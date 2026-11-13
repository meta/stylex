use super::super::*;
use swc_core::ecma::ast::AwaitExpr;

pub(in super::super) fn evaluate(
  await_expr: &AwaitExpr,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Option<EvaluateResultValue> {
  evaluate_cached(&await_expr.arg, state, traversal_state, fns)
}
