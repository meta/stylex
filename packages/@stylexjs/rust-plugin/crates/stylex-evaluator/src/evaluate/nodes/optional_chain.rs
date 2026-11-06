use super::super::*;
use swc_core::ecma::ast::OptChainExpr;

pub(in super::super) fn evaluate(
  opt_chain: &OptChainExpr,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Option<EvaluateResultValue> {
  if matches!(opt_chain.base.as_ref(), OptChainBase::Call(_)) {
    return deopt(
      &Expr::OptChain(opt_chain.clone()),
      state,
      &unsupported_expression("OptionalCallExpression"),
    );
  }

  let base_result = match opt_chain.base.as_ref() {
    OptChainBase::Member(member) => evaluate_cached(&member.obj, state, traversal_state, fns),
    OptChainBase::Call(call) => evaluate_cached(&call.callee, state, traversal_state, fns),
  };

  let should_short_circuit = match &base_result {
    Some(EvaluateResultValue::Expr(base_expr)) => {
      matches!(base_expr, Expr::Lit(Lit::Null(_)))
        || (matches!(base_expr, Expr::Ident(ident) if is_js_undefined(ident)))
    },
    None => true,

    _ => false,
  };

  if should_short_circuit {
    None
  } else {
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
