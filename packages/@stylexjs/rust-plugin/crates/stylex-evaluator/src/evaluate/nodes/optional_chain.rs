use super::super::*;
use swc_core::ecma::ast::OptChainExpr;

pub(in super::super) fn evaluate(
  opt_chain: &OptChainExpr,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Option<EvaluateResultValue> {
  // Evaluate the base object/callee first
  let base_result = match opt_chain.base.as_ref() {
    OptChainBase::Member(member) => evaluate_cached(&member.obj, state, traversal_state, fns),
    OptChainBase::Call(call) => evaluate_cached(&call.callee, state, traversal_state, fns),
  };

  // Check if we should short-circuit:
  // 1. Base is null literal
  // 2. Base is undefined identifier
  // 3. Base evaluation failed (returned None)
  let should_short_circuit = match &base_result {
    Some(EvaluateResultValue::Expr(base_expr)) => {
      matches!(base_expr, Expr::Lit(Lit::Null(_)))
        || (matches!(base_expr, Expr::Ident(ident) if is_js_undefined(ident)))
    },
    None => true,
    // For other result types (Object, Array, FunctionConfig, etc.), don't short-circuit
    _ => false,
  };

  if should_short_circuit {
    None
  } else {
    // Otherwise, evaluate the full optional chain expression
    match opt_chain.base.as_ref() {
      OptChainBase::Member(member) => {
        let member_expr = Expr::Member(member.clone());
        evaluate_cached(&member_expr, state, traversal_state, fns)
      },
      OptChainBase::Call(call) => {
        let call_expr = Expr::Call(call.clone().into());
        evaluate_cached(&call_expr, state, traversal_state, fns)
      },
    }
  }
}

#[cfg(test)]
#[path = "tests/optional_chain_tests.rs"]
mod optional_chain_tests;
