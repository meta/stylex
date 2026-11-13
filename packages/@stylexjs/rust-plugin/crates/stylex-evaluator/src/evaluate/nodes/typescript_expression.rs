use super::super::*;

pub(in super::super) fn evaluate(
  expr: &Expr,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Option<EvaluateResultValue> {
  evaluate_cached(expr, state, traversal_state, fns)
}
