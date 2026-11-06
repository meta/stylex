use super::super::*;
/// Cooked template contents stay UTF-16 through every intermediate operation; lone surrogates are never sent through a Rust-string conversion here.
pub(in super::super) fn evaluate_quasis(
  path: &Expr,
  exprs: &[Box<Expr>],
  quasis: &[TplElement],
  state: &mut EvaluationState,
  traversal: &mut StateManager,
  fns: &FunctionMap,
) -> Option<EvaluateResultValue> {
  let mut output = Vec::new();
  for (index, quasi) in quasis.iter().enumerate() {
    let Some(cooked) = quasi.cooked.as_ref() else {
      return deopt(path, state, EXPRESSION_IS_NOT_A_STRING);
    };
    let units = stylex_js::native::wtf_units(cooked.code_points().map(|point| point.to_u32()));
    let limit = traversal.character_ceiling();
    let observed = output.len().saturating_add(units.len());
    if observed > limit {
      let _: Option<()> =
        stylex_js::native::refuse_quota(stylex_js::native::QuotaKind::Characters, limit, observed);
      return deopt(
        path,
        state,
        &grown_string_too_large(
          stylex_constants::constants::evaluation_errors::TEMPLATE_LITERAL,
          limit as u64,
        ),
      );
    }
    if stylex_js::native::charge_output(units.len().saturating_mul(2)).is_none() {
      return deopt(
        path,
        state,
        "Static evaluation exceeded its resource limit.",
      );
    }
    output.extend(units);
    if let Some(expression) = exprs.get(index) {
      let value = evaluate_cached(expression, state, traversal, fns)?;

      if matches!(value, EvaluateResultValue::FunctionConfig(_)) {
        return deopt(path, state, EXPRESSION_IS_NOT_A_STRING);
      }
      let units = match super::super::primitive::string_units(&value, traversal, path.span()) {
        Ok(units) => units,
        Err(error) => {
          if let Some(refusal) = stylex_js::native::quota_refusal() {
            return deopt(
              path,
              state,
              &quota_reason(
                refusal,
                QuotaBoundary::Growth(
                  stylex_constants::constants::evaluation_errors::TEMPLATE_LITERAL,
                ),
              ),
            );
          }
          return super::super::callback_failure::invocation_failure(error, state);
        },
      };
      let limit = traversal.character_ceiling();
      let observed = output.len().saturating_add(units.len());
      if observed > limit {
        let _: Option<()> = stylex_js::native::refuse_quota(
          stylex_js::native::QuotaKind::Characters,
          limit,
          observed,
        );
        return deopt(
          path,
          state,
          &grown_string_too_large(
            stylex_constants::constants::evaluation_errors::TEMPLATE_LITERAL,
            limit as u64,
          ),
        );
      }
      if stylex_js::native::charge_output(units.len().saturating_mul(2)).is_none() {
        return deopt(
          path,
          state,
          "Static evaluation exceeded its resource limit.",
        );
      }
      output.extend(units);
    }
  }
  Some(EvaluateResultValue::Expr(
    super::super::host_values::utf16_expr(&output),
  ))
}
#[cfg(test)]
#[path = "tests/template_interpolation_tests.rs"]
mod template_interpolation_tests;
