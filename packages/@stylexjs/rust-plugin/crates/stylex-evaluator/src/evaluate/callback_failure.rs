use super::*;
use std::cell::Cell;
use stylex_state::types::EvaluationFailure;
thread_local! {static CAUSAL_INVOCATIONS:Cell<usize>=const{Cell::new(0)};}
/// A registered compiler callback returns a typed failure through HostEnv;
/// ordinary evaluation keeps its established plain-arrow panic transport.
pub(super) struct CausalInvocation {
  previous: usize,
}
impl CausalInvocation {
  pub(super) fn enter() -> Self {
    let previous = CAUSAL_INVOCATIONS.with(|scope| {
      let previous = scope.get();
      scope.set(previous.saturating_add(1));
      previous
    });
    Self { previous }
  }
}
impl Drop for CausalInvocation {
  fn drop(&mut self) {
    CAUSAL_INVOCATIONS.with(|scope| scope.set(self.previous));
  }
}
pub(super) fn in_causal_invocation() -> bool {
  CAUSAL_INVOCATIONS.with(|scope| scope.get() != 0)
}
pub(crate) fn with_isolated_causal_failures<T>(operation: impl FnOnce() -> T) -> T {
  let previous = CAUSAL_INVOCATIONS.with(|scope| scope.replace(0));
  let _restore = CausalInvocation { previous };
  operation()
}
/// Expression-arrow errors preserve typed kind/message/span. Capability and
/// resource refusals are not author exceptions. No message classification.
pub(super) fn invocation_failure(
  failure: EvaluationFailure,
  state: &mut EvaluationState,
) -> Option<EvaluateResultValue> {
  if failure.plain && !in_causal_invocation() {
    stylex_macros::stylex_error::mark_panic_plain();
    stylex_macros::stylex_panic!("{}", failure.message);
  }
  let expression = Expr::Invalid(swc_core::ecma::ast::Invalid { span: failure.span });
  deopt(&expression, state, &failure.message)
}
