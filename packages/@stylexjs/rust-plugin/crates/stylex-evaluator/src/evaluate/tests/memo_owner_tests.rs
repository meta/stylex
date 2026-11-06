//! Memo entries require retained immutable owners and never merge allocations.
use super::{EvaluationState, Expr, FunctionMap, Memoized, StateManager, memoized_fold};
use stylex_ast::ast::{
  convertors::create_number_expr,
  factories::{create_ident, create_var_declarator},
};
use stylex_state::evaluate_result_value::EvaluateResultValue;
use swc_core::{common::DUMMY_SP, ecma::ast::ObjectLit};
fn object() -> Expr {
  Expr::Object(ObjectLit {
    span: DUMMY_SP,
    props: Vec::new(),
  })
}
fn math() -> Expr {
  Expr::Bin(swc_core::ecma::ast::BinExpr {
    span: DUMMY_SP,
    op: swc_core::ecma::ast::BinaryOp::Add,
    left: Box::new(create_number_expr(1.0)),
    right: Box::new(create_number_expr(2.0)),
  })
}
#[test]
fn memo_owned_key_is_not_written_after_its_initializer_is_rewritten() {
  let mut traversal = StateManager::default();
  let id = create_ident("value");
  traversal.push_declaration(create_var_declarator(id.clone(), math()));
  let (_, owner) = traversal.declaration_parts(&id).unwrap();
  let owner = owner.unwrap();
  let value = memoized_fold(
    &owner,
    &mut EvaluationState::default(),
    &mut traversal,
    |_, traversal| {
      traversal.set_declaration_init(0, create_number_expr(4.0));
      Some(EvaluateResultValue::Expr(create_number_expr(3.0)))
    },
  );
  assert!(value.is_some());
  assert!(
    traversal.seen.is_empty(),
    "a revision change must leave no dangling-owner memo entry"
  );
}
#[test]
fn memo_allocations_are_not_stored_as_one_shared_instance() {
  let mut traversal = StateManager::default();
  let path = math();
  let value = memoized_fold(
    &path,
    &mut EvaluationState::default(),
    &mut traversal,
    |_, _| Some(EvaluateResultValue::Expr(object())),
  );
  assert!(value.is_some());
  assert!(traversal.seen.is_empty());
  assert!(
    !EvaluateResultValue::Vec(vec![EvaluateResultValue::Expr(create_number_expr(1.0))])
      .can_memoize()
  );
}
#[test]
fn memo_effect_nodes_leave_function_fingerprinting_lazy() {
  let mut traversal = StateManager::default();
  let mut state = EvaluationState::default();
  assert!(state.memo_inventory.is_none());
  super::evaluate_cached(
    &create_number_expr(1.0),
    &mut state,
    &mut traversal,
    &FunctionMap::default(),
  )
  .unwrap();
  assert!(
    state.memo_inventory.is_none(),
    "one-shot literal must not hash the function inventory"
  );
  let call = Expr::Call(swc_core::ecma::ast::CallExpr {
    span: DUMMY_SP,
    ctxt: Default::default(),
    callee: swc_core::ecma::ast::Callee::Expr(Box::new(Expr::Ident(create_ident("unknown")))),
    args: Vec::new(),
    type_args: None,
  });
  let mut call_state = EvaluationState::default();
  super::evaluate_cached(
    &call,
    &mut call_state,
    &mut traversal,
    &FunctionMap::default(),
  );
  assert!(
    call_state.memo_inventory.is_none(),
    "effect-capable call/get/identifier path must not fingerprint"
  );
  let mut pure_state = EvaluationState::default();
  memoized_fold(&math(), &mut pure_state, &mut traversal, |_, _| {
    Some(EvaluateResultValue::Expr(create_number_expr(3.0)))
  })
  .unwrap();
  assert!(
    pure_state.memo_inventory.is_some(),
    "pure AST memo still retains its exact inventory and stays enabled"
  );
}

#[test]
fn memo_current_host_epoch_invalidates_a_warmed_primitive() {
  stylex_js::native::with_evaluation(|| {
    let mut traversal = StateManager::default();
    let path = math();
    let mut folds = 0;
    for changed in [false, true] {
      if changed {
        stylex_js::native::observe_host_operation();
      }
      memoized_fold(
        &path,
        &mut EvaluationState::default(),
        &mut traversal,
        |_, _| {
          folds += 1;
          Some(EvaluateResultValue::Expr(create_number_expr(f64::from(
            folds,
          ))))
        },
      )
      .unwrap();
    }
    assert_eq!(
      folds, 2,
      "a memo saved before a live host mutation cannot serve a later read"
    );
  });
}

#[test]
fn memo_actual_host_revision_survives_separate_budget_scopes() {
  stylex_js::native::with_isolated_evaluation(|| {
    let mut traversal = StateManager::default();
    let path = math();
    let mut folds = 0;
    for changed in [false, true] {
      if changed {
        assert!(!stylex_js::native::has_active_evaluation());
        stylex_js::native::observe_host_operation();
        assert_eq!(stylex_js::native::host_epoch(), 0);
      }
      stylex_js::native::with_evaluation(|| {
        memoized_fold(
          &path,
          &mut EvaluationState::default(),
          &mut traversal,
          |_, _| {
            folds += 1;
            Some(EvaluateResultValue::Expr(create_number_expr(f64::from(
              folds,
            ))))
          },
        )
        .unwrap();
      });
    }
    assert_eq!(
      folds, 2,
      "host effects outside ACTIVE must invalidate a prior budget's memo"
    );
  });
}

#[test]
fn memo_literal_repeat_admits_only_inert_receiver_and_count() {
  use swc_core::ecma::ast::{CallExpr, Callee, ExprOrSpread, MemberExpr, MemberProp};
  let mut call = CallExpr {
    span: DUMMY_SP,
    ctxt: Default::default(),
    callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
      span: DUMMY_SP,
      obj: Box::new(stylex_ast::ast::convertors::create_string_expr("x")),
      prop: MemberProp::Ident(stylex_ast::ast::factories::create_ident_name("repeat")),
    }))),
    args: vec![ExprOrSpread {
      spread: None,
      expr: Box::new(create_number_expr(250000.0)),
    }],
    type_args: None,
  };
  assert!(super::inert_literal_repeat(&call));
  *call.args[0].expr = Expr::Ident(create_ident("count"));
  assert!(
    !super::inert_literal_repeat(&call),
    "a binding must evaluate before any hit"
  );
  *call.args[0].expr = create_number_expr(1.0);
  call.args.push(ExprOrSpread {
    spread: None,
    expr: Box::new(Expr::Ident(create_ident("extra"))),
  });
  assert!(
    !super::inert_literal_repeat(&call),
    "additional arguments cannot hide reads"
  );
  call.args.pop();
  let Callee::Expr(callee) = &mut call.callee else {
    unreachable!()
  };
  let Expr::Member(member) = &mut **callee else {
    unreachable!()
  };
  *member.obj = Expr::Ident(create_ident("receiver"));
  assert!(
    !super::inert_literal_repeat(&call),
    "alias/live receiver must stay uncached"
  );
}

#[test]
fn memo_primitive_results_remain_enabled() {
  let mut traversal = StateManager::default();
  let path = math();
  let mut folds = 0;
  for _ in 0..2 {
    memoized_fold(
      &path,
      &mut EvaluationState::with_functions(std::rc::Rc::new(FunctionMap::default())),
      &mut traversal,
      |_, _| {
        folds += 1;
        Some(EvaluateResultValue::Expr(create_number_expr(3.0)))
      },
    )
    .unwrap();
  }
  assert_eq!(folds, 1);
  assert_eq!(traversal.seen.len(), 1);
}
