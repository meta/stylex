//! Primitive literals cost fuel but no recursive AST level or memo work.
use super::{EvaluationState, Expr, FunctionMap, StateManager, evaluate_cached};
use crate::evaluate::source_evaluation::evaluate_source_with_ceiling;
use stylex_ast::ast::convertors::create_number_expr;

#[test]
fn primitive_leaf_receiver_does_not_make_a_one_level_trim_call_fail() {
  let result = evaluate_source_with_ceiling("' x '.trim()", 1);
  assert!(result.confident, "{:?}", result.reason);
  let value = result
    .value
    .as_ref()
    .and_then(|value| value.as_expr())
    .and_then(|expr| expr.as_lit())
    .and_then(|lit| lit.as_str());
  assert_eq!(value.and_then(|value| value.value.as_str()), Some("x"));
}
#[test]
fn primitive_leaf_respects_an_already_refused_state() {
  let mut state = EvaluationState::default();
  let mut traversal = StateManager::default();
  let expr = create_number_expr(3.0);
  assert!(evaluate_cached(&expr, &mut state, &mut traversal, &FunctionMap::default()).is_some());
  state.confident = false;
  assert!(evaluate_cached(&expr, &mut state, &mut traversal, &FunctionMap::default()).is_none());
}
#[test]
fn primitive_leaf_cannot_refund_or_ignore_exhausted_work() {
  stylex_js::native::with_evaluation(|| {
    let _: Option<()> = stylex_js::native::refuse_resource();
    let mut state = EvaluationState::default();
    let value = evaluate_cached(
      &create_number_expr(3.0),
      &mut state,
      &mut StateManager::default(),
      &FunctionMap::default(),
    );
    assert!(value.is_none());
    assert!(!state.confident);
  });
}
#[test]
fn primitive_leaf_preserves_wtf8_and_unsupported_literal_refusals() {
  let result = evaluate_source_with_ceiling("'\\uD800'", 1);
  let Some(Expr::Lit(swc_core::ecma::ast::Lit::Str(value))) =
    result.value.as_ref().and_then(|value| value.as_expr())
  else {
    panic!("string literal must retain its value")
  };
  assert_eq!(
    value
      .value
      .code_points()
      .map(|point| point.to_u32())
      .collect::<Vec<_>>(),
    vec![0xD800]
  );
  for source in ["1n", "/x/"] {
    assert!(
      !evaluate_source_with_ceiling(source, 8).confident,
      "unsupported literal accepted: {source}"
    );
  }
}
