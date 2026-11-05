use super::*;
use crate::utils::parser::parse_module;
use crate::utils::transform::bindings::collect_local_expressions;
#[derive(Default)]
struct Store {
    saved: Option<(OwnedEvaluationScope, EvalValue)>,
}
impl EvaluationHooks for Store {
    fn call(
        &mut self,
        f: &EvalValue,
        args: &[EvalValue],
        scope: EvaluationScope<'_>,
    ) -> HookResult {
        if matches!(f, EvalValue::OpaqueFunction(7)) {
            let Some(function) = args.first() else {
                return HookResult::Deopt;
            };
            let Some(snapshot) = scope.snapshot() else {
                return HookResult::Deopt;
            };
            self.saved = Some((snapshot, function.clone()));
            HookResult::Value(EvalValue::Number(1.0))
        } else {
            HookResult::Unhandled
        }
    }
}
#[test]
fn owned_host_closure_scope_keeps_callback_parameters_and_original_budget() {
    let parsed = parse_module("[3].map(v=>store(()=>v+1));", "snapshot.js").unwrap();
    let expressions = collect_local_expressions(&parsed.module);
    let safety = collect_binding_safety_with_imports(
        &parsed.module,
        &crate::shared::CollectedImports::default(),
    );
    let values = HashMap::from([(
        ("store".into(), Default::default()),
        EvalValue::OpaqueFunction(7),
    )]);
    let ModuleItem::Stmt(Stmt::Expr(e)) = &parsed.module.body[0] else {
        panic!()
    };
    let mut hooks = Store::default();
    assert!(evaluate_static_expr_with_hooks_and_safety_result(
        &e.expr,
        &values,
        &expressions,
        &safety,
        &mut hooks
    )
    .is_ok());
    let (snapshot, function) = hooks.saved.take().unwrap();
    assert_eq!(
        snapshot.call(&function, vec![], &mut Store::default()),
        Ok(EvalValue::Number(4.0))
    );
    let mut rejected = false;
    for _ in 0..MAX_FUEL {
        if snapshot
            .call(&function, vec![], &mut Store::default())
            .is_err()
        {
            rejected = true;
            break;
        }
    }
    assert!(
        rejected,
        "stored callback must not reset fuel each time it is invoked"
    );
}
#[test]
fn input_budget_scaling_is_monotonic_not_replenished() {
    let budget = Budget::new();
    budget.observe_input_bytes(100_000);
    budget.spend(99).unwrap();
    let left = budget.fuel.get();
    budget.observe_input_bytes(100_000);
    assert_eq!(budget.fuel.get(), left);
    budget.observe_input_bytes(10);
    assert_eq!(budget.fuel.get(), left);
    budget.observe_input_bytes(100_001);
    assert_eq!(budget.fuel.get(), left + 8);
}
#[test]
fn detailed_failures_preserve_mutation_tdz_and_blocked_spans() {
    struct Empty;
    impl EvaluationHooks for Empty {}
    for (code, message) in [
        ("let a=1;a=2;a;", "Referenced value is not a constant.\n\n"),
        (
            "const a=[1];a.push(2);a[0];",
            "Referenced value is not a constant.\n\n",
        ),
        (
            "a;const a=1;",
            "Referenced value is used before declaration.\n\n",
        ),
        ("this;", "Unsupported expression: ThisExpression\n\n"),
    ] {
        let parsed = parse_module(code, "failure.js").unwrap();
        let mut expressions = collect_local_expressions(&parsed.module);
        let safety = collect_binding_safety_with_imports(
            &parsed.module,
            &crate::shared::CollectedImports::default(),
        );
        let mut values = HashMap::new();
        safety.invalidate(&mut values, &mut expressions);
        let e = parsed
            .module
            .body
            .iter()
            .find_map(|i| {
                if let ModuleItem::Stmt(Stmt::Expr(e)) = i {
                    if matches!(&*e.expr, Expr::Ident(_) | Expr::Member(_) | Expr::This(_)) {
                        Some(&*e.expr)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .unwrap();
        let error = evaluate_static_expr_with_hooks_and_safety_result(
            e,
            &HashMap::new(),
            &expressions,
            &safety,
            &mut Empty,
        )
        .unwrap_err();
        assert_eq!(error.message, message, "{code}");
        assert_ne!(error.span.lo.0, 0);
    }
    let parsed = parse_module("({}).constructor;", "blocked.js").unwrap();
    let ModuleItem::Stmt(Stmt::Expr(e)) = &parsed.module.body[0] else {
        panic!()
    };
    let error = evaluate_static_expr_with_hooks_and_safety_result(
        &e.expr,
        &HashMap::new(),
        &HashMap::new(),
        &BindingSafety::default(),
        &mut Empty,
    )
    .unwrap_err();
    assert_eq!(
        error.message,
        EvaluationFailure::blocked(error.span).message
    );
    assert_eq!(
        error.span.hi.0 - error.span.lo.0,
        "constructor".len() as u32
    );
}
