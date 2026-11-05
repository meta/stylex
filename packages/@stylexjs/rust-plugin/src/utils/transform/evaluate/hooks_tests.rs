use super::*;
use crate::utils::parser::parse_module;
use crate::utils::transform::bindings::collect_local_expressions;
use std::cell::Cell;
struct Hooks {
    calls: Vec<f64>,
    probe: Option<Expr>,
    saw_overlay: Cell<bool>,
}
impl EvaluationHooks for Hooks {
    fn member(&mut self, object: &EvalValue, key: &str, _scope: EvaluationScope<'_>) -> HookResult {
        match (object, key) {
            (EvalValue::OpaqueObject(0), "double") => {
                HookResult::Value(EvalValue::OpaqueFunction(1))
            }
            (EvalValue::OpaqueObject(2), "color") => {
                HookResult::Value(EvalValue::String("red".into()))
            }
            _ => HookResult::Unhandled,
        }
    }
    fn call(
        &mut self,
        callee: &EvalValue,
        args: &[EvalValue],
        scope: EvaluationScope<'_>,
    ) -> HookResult {
        if matches!(callee, EvalValue::OpaqueFunction(1)) {
            let Some(EvalValue::Number(n)) = args.first() else {
                return HookResult::Deopt;
            };
            let n = *n;
            if let Some(probe) = self.probe.clone() {
                let value = evaluate_static_expr_in_scope_with_hooks(&probe, scope, self);
                self.saw_overlay.set(value == Some(EvalValue::Number(n)));
            }
            self.calls.push(n);
            HookResult::Value(EvalValue::Number(n * 2.0))
        } else if matches!(callee, EvalValue::OpaqueFunction(3)) {
            self.calls.push(99.0);
            HookResult::Value(EvalValue::OpaqueObject(2))
        } else {
            HookResult::Unhandled
        }
    }
    fn to_primitive(
        &mut self,
        value: &EvalValue,
        _hint: PrimitiveHint,
        _scope: EvaluationScope<'_>,
    ) -> HookResult {
        if matches!(value, EvalValue::OpaqueObject(2)) {
            HookResult::Value(EvalValue::String("theme".into()))
        } else {
            HookResult::Unhandled
        }
    }
}
fn evaluate(code: &str, hooks: &mut Hooks) -> Option<EvalValue> {
    let parsed = parse_module(code, "hook.js").unwrap();
    let mut expressions = collect_local_expressions(&parsed.module);
    let safety = collect_binding_safety_with_imports(
        &parsed.module,
        &crate::shared::CollectedImports::default(),
    );
    let mut plain = HashMap::new();
    safety.invalidate(&mut plain, &mut expressions);
    let mut values = HashMap::new();
    values.insert(
        ("env".into(), Default::default()),
        EvalValue::OpaqueObject(0),
    );
    values.insert(
        ("getTheme".into(), Default::default()),
        EvalValue::OpaqueFunction(3),
    );
    let ModuleItem::Stmt(Stmt::Expr(e)) = parsed.module.body.last()? else {
        return None;
    };
    evaluate_static_expr_with_hooks_and_safety_result(
        &e.expr,
        &values,
        &expressions,
        &safety,
        hooks,
    )
    .ok()
}
fn hooks() -> Hooks {
    Hooks {
        calls: vec![],
        probe: None,
        saw_overlay: Cell::new(false),
    }
}
#[test]
fn native_calls_see_real_arrow_overlay_and_are_not_eager() {
    let mut h = hooks();
    let parsed = parse_module("[1].map(v=>env.double(v));", "hook.js").unwrap();
    struct Probe(Option<Expr>);
    impl swc_ecma_visit::Visit for Probe {
        fn visit_call_expr(&mut self, c: &CallExpr) {
            if matches!(&c.callee,Callee::Expr(e)if matches!(&**e,Expr::Member(_))) {
                if let Some(arg) = c.args.first() {
                    if matches!(&*arg.expr,Expr::Ident(i)if i.sym==*"v") {
                        self.0 = Some((*arg.expr).clone());
                    }
                }
            }
            swc_ecma_visit::VisitWith::visit_children_with(c, self);
        }
    }
    let mut probe = Probe(None);
    swc_ecma_visit::VisitWith::visit_with(&parsed.module, &mut probe);
    h.probe = probe.0;
    assert_eq!(
        evaluate("[1,2].map(v=>env.double(v)).join(',');", &mut h),
        Some(EvalValue::String("2,4".into()))
    );
    assert_eq!(h.calls, vec![1.0, 2.0]);
    assert!(h.saw_overlay.get());
    h.calls.clear();
    assert_eq!(
        evaluate("false && env.double(3);", &mut h),
        Some(EvalValue::Bool(false))
    );
    assert!(h.calls.is_empty());
    assert_eq!(
        evaluate("void env.double(3);", &mut h),
        Some(EvalValue::Undefined)
    );
    assert!(h.calls.is_empty());
}
#[test]
fn safe_binding_memo_is_scoped_and_composes_opaque_values() {
    let mut h = hooks();
    assert_eq!(
        evaluate("const theme=getTheme(); theme.color + theme.color;", &mut h),
        Some(EvalValue::String("redred".into()))
    );
    assert_eq!(h.calls, vec![99.0]);
    h.calls.clear();
    assert_eq!(
        evaluate("`${getTheme()}`;", &mut h),
        Some(EvalValue::String("theme".into()))
    );
    assert_eq!(h.calls, vec![99.0]);
    h.calls.clear();
    assert_eq!(
        evaluate("getTheme().color.length;", &mut h),
        Some(EvalValue::Number(3.0))
    );
    assert_eq!(h.calls, vec![99.0]);
}
#[test]
fn hook_reentry_cannot_reset_budget_or_bypass_invalid() {
    struct Recursive;
    impl EvaluationHooks for Recursive {
        fn resolve(&mut self, e: &Expr, s: EvaluationScope<'_>) -> HookResult {
            evaluate_static_expr_in_scope_with_hooks(e, s, self)
                .map(HookResult::Value)
                .unwrap_or(HookResult::Deopt)
        }
    }
    let parsed = parse_module("missing;", "hook.js").unwrap();
    let ModuleItem::Stmt(Stmt::Expr(e)) = &parsed.module.body[0] else {
        panic!()
    };
    assert!(evaluate_static_expr_with_hooks_and_safety_result(
        &e.expr,
        &HashMap::new(),
        &HashMap::new(),
        &BindingSafety::default(),
        &mut Recursive
    )
    .is_err());
    let mut h = hooks();
    assert_eq!(
        evaluate("const theme=getTheme();unknown(theme);theme.color;", &mut h),
        Some(EvalValue::String("red".into()))
    );
    assert_eq!(h.calls, vec![99.0]);
    h.calls.clear();
    assert!(evaluate(
        "const theme=getTheme();theme.color='blue';theme.color;",
        &mut h
    )
    .is_none());
    assert!(h.calls.is_empty());
}
#[test]
fn block_locals_obey_tdz_even_when_function_is_called_later() {
    let mut h = hooks();
    assert!(evaluate("const f=()=>{const a=b;const b=1;return a;};f();", &mut h).is_none());
}
