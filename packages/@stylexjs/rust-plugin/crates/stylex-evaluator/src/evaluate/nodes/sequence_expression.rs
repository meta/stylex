use super::super::*;
use swc_core::ecma::ast::SeqExpr;

/// A sequence is worth its last operand, which is what the language answers.
///
/// A sequence with no operands at all cannot be written -- the comma is what
/// makes one, so the grammar gives every sequence at least two. It is answered
/// here rather than refused, and answered by the same `and_then` that carries
/// the operand: an arm of its own would be a claim no case could enter, and the
/// caller above turns an absent answer into the refusal that names the node.
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
