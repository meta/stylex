use super::super::*;
use swc_core::ecma::ast::SeqExpr;

/// A sequence is worth its last operand, which is what the language answers.
pub(in super::super) fn evaluate(
  sec: &SeqExpr,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Option<EvaluateResultValue> {
  sec
    .exprs
    .last()
    .and_then(|expr| evaluate_cached(expr, state, traversal_state, fns))
}
