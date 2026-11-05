use super::*;
use crate::utils::parser::parse_module;
use crate::utils::transform::bindings::collect_local_expressions;
#[derive(Default)]
struct Identity {
    seen: Vec<Rc<EvalFunction>>,
}
impl EvaluationHooks for Identity {
    fn call(
        &mut self,
        value: &EvalValue,
        args: &[EvalValue],
        _scope: EvaluationScope<'_>,
    ) -> HookResult {
        match value {
            EvalValue::OpaqueFunction(1) => {
                let [EvalValue::Function(left), EvalValue::Function(right)] = args else {
                    return HookResult::Deopt;
                };
                HookResult::Value(EvalValue::Number(if Rc::ptr_eq(left, right) {
                    1.0
                } else {
                    0.0
                }))
            }
            EvalValue::OpaqueFunction(2) => {
                let [EvalValue::Function(function)] = args else {
                    return HookResult::Deopt;
                };
                self.seen.push(function.clone());
                HookResult::Value(EvalValue::Number(1.0))
            }
            _ => HookResult::Unhandled,
        }
    }
}
fn evaluate(code: &str, hooks: &mut Identity) -> Option<EvalValue> {
    let parsed = parse_module(code, "identity.js").unwrap();
    let mut expressions = collect_local_expressions(&parsed.module);
    let safety = collect_binding_safety_with_imports(
        &parsed.module,
        &crate::shared::CollectedImports::default(),
    );
    let mut plain = HashMap::new();
    safety.invalidate(&mut plain, &mut expressions);
    let values = HashMap::from([
        (
            ("same".into(), Default::default()),
            EvalValue::OpaqueFunction(1),
        ),
        (
            ("observe".into(), Default::default()),
            EvalValue::OpaqueFunction(2),
        ),
    ]);
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
#[test]
fn repeated_binding_and_alias_reads_keep_one_function_instance() {
    let mut hooks = Identity::default();
    assert_eq!(
        evaluate("const a=x=>x; same(a,a);", &mut hooks),
        Some(EvalValue::Number(1.0))
    );
    assert_eq!(
        evaluate("const a=x=>x; const b=a; same(a,b);", &mut hooks),
        Some(EvalValue::Number(1.0))
    );
    assert_eq!(
        evaluate("const a=x=>x; [1,2].map(()=>observe(a));", &mut hooks),
        Some(array(vec![EvalValue::Number(1.0), EvalValue::Number(1.0)]))
    );
    assert_eq!(hooks.seen.len(), 2);
    assert!(
        Rc::ptr_eq(&hooks.seen[0], &hooks.seen[1]),
        "root binding must not be re-instantiated in each callback frame"
    );
}
#[test]
fn binding_instance_memo_does_not_cross_root_evaluations_or_override_parameters() {
    let mut hooks = Identity::default();
    assert!(evaluate("const a=x=>x;observe(a);", &mut hooks).is_some());
    assert!(evaluate("const a=x=>x;observe(a);", &mut hooks).is_some());
    assert_eq!(hooks.seen.len(), 2);
    assert!(
        !Rc::ptr_eq(&hooks.seen[0], &hooks.seen[1]),
        "separate roots must not share allocated bindings"
    );
    hooks.seen.clear();
    assert!(evaluate(
        "const a=()=>0; [1,2].map(v=>((a)=>observe(a))(()=>v));",
        &mut hooks
    )
    .is_some());
    assert_eq!(hooks.seen.len(), 2);
    assert!(!Rc::ptr_eq(&hooks.seen[0],&hooks.seen[1]),"actual parameter functions must not be replaced by a memoized same-spelling source binding");
}
#[test]
fn arrow_literals_and_callback_local_declarations_are_fresh_per_invocation() {
    let mut hooks = Identity::default();
    assert_eq!(
        evaluate("[1,2].map(v=>observe(()=>v));", &mut hooks),
        Some(array(vec![EvalValue::Number(1.0), EvalValue::Number(1.0)]))
    );
    assert_eq!(hooks.seen.len(), 2);
    assert!(!Rc::ptr_eq(&hooks.seen[0], &hooks.seen[1]));
    hooks.seen.clear();
    assert_eq!(
        evaluate(
            "[1,2].map(v=>{const a=()=>v;return observe(a);});",
            &mut hooks
        ),
        Some(array(vec![EvalValue::Number(1.0), EvalValue::Number(1.0)]))
    );
    assert_eq!(hooks.seen.len(), 2);
    assert!(!Rc::ptr_eq(&hooks.seen[0], &hooks.seen[1]));
    assert_eq!(
        evaluate(
            "[1,2].map(v=>{const a=()=>v;return same(a,a);}).join(',');",
            &mut hooks
        ),
        Some(EvalValue::String("1,1".into()))
    );
}
