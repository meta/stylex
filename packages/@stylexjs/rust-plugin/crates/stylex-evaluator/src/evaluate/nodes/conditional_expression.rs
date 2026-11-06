use super::super::*;
use swc_core::ecma::ast::CondExpr;

pub(in super::super) fn evaluate(
  cond: &CondExpr,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Option<EvaluateResultValue> {
  let test_value = evaluate_cached(&cond.test, state, traversal_state, fns)?;

  let takes_the_consequent = match super::super::truthiness::truthy(&test_value, traversal_state) {
    Ok(value) => value,
    Err(error) => return deopt(&Expr::Cond(cond.clone()), state, &error.message),
  };

  match takes_the_consequent {
    true => evaluate_cached(&cond.cons, state, traversal_state, fns),
    false => evaluate_cached(&cond.alt, state, traversal_state, fns),
  }
}
