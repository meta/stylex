use super::*;
use stylex_structures::stylex_env::{HostError, HostResult, HostValue};

pub(super) fn binary_values(
  op: swc_core::ecma::ast::BinaryOp,
  left: &EvaluateResultValue,
  right: &EvaluateResultValue,
  traversal: &StateManager,
) -> HostResult<EvaluateResultValue> {
  let Some(environment) = traversal.host_environment() else {
    return Err(HostError {
      message: "JavaScript host is not configured.".into(),
    });
  };
  let left = super::host_dispatch::argument(left, traversal, environment.host.clone())?;
  let right = super::host_dispatch::argument(right, traversal, environment.host.clone())?;
  stylex_js::native::observe_host_operation();
  Ok(super::host_dispatch::host_result(
    environment.host.binary(op.as_str(), &left, &right)?,
    None,
  ))
}
pub(super) fn unary_value(
  op: swc_core::ecma::ast::UnaryOp,
  value: &EvaluateResultValue,
  traversal: &StateManager,
) -> HostResult<EvaluateResultValue> {
  let Some(environment) = traversal.host_environment() else {
    return Err(HostError {
      message: "JavaScript host is not configured.".into(),
    });
  };
  let value = super::host_dispatch::argument(value, traversal, environment.host.clone())?;
  if op == swc_core::ecma::ast::UnaryOp::TypeOf
    && let HostValue::Ref(reference) = value
  {
    return Ok(EvaluateResultValue::Expr(create_string_expr(
      environment.host.type_of(&reference),
    )));
  }
  stylex_js::native::observe_host_operation();
  Ok(super::host_dispatch::host_result(
    environment.host.unary(op.as_str(), &value)?,
    None,
  ))
}
