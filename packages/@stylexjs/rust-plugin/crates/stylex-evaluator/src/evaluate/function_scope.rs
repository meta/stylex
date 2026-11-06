use super::*;
use stylex_state::types::{EvaluationCallback, EvaluationFailure};
use swc_core::{common::Span, ecma::ast::Id};
pub fn evaluate_function_in_scope(
  function: &EvaluationCallback,
  args: Vec<EvaluateResultValue>,
  state: &mut StateManager,
  span: Span,
  overlays: &rustc_hash::FxHashMap<Id, EvaluateResultValue>,
) -> Result<EvaluateResultValue, EvaluationFailure> {
  evaluate_function_in_scope_observed(function, args, state, span, overlays, None)
}
/// Install observation only after entering the stored counter ledger.
pub fn evaluate_function_in_scope_observed(
  function: &EvaluationCallback,
  args: Vec<EvaluateResultValue>,
  state: &mut StateManager,
  span: Span,
  overlays: &rustc_hash::FxHashMap<Id, EvaluateResultValue>,
  observer: Option<MemberReadObserver>,
) -> Result<EvaluateResultValue, EvaluationFailure> {
  let operation = || {
    let invoke = || {
      stylex_js::native::with_callsite(span.lo.0, || {
        (function.invoke_with_overlays)(args, state, overlays)
      })
    };
    match observer {
      Some(observer) => super::member_observer::with_member_read_observer(observer, invoke),
      None => invoke(),
    }
  };
  match function.budget.as_ref().and_then(|lease| lease.upgrade()) {
    Some(budget) => stylex_js::native::with_captured_evaluation(&budget, operation),
    None => stylex_js::native::with_evaluation(operation),
  }
}
