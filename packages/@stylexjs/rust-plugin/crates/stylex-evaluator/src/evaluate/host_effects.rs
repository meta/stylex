use super::*;
use stylex_state::{state_manager::evaluation_scope::ScopedEffects, types::EvaluationFailure};
use stylex_structures::stylex_env::{HostEnv, HostError, HostResult};
/// Own the effect collector, never a borrowed StateManager pointer. Immutable
/// Get/materialize APIs queue work until a mutable evaluator/writer boundary.
pub(super) fn operation<T>(
  state: &StateManager,
  host: Rc<dyn HostEnv>,
  operation: impl FnOnce() -> HostResult<T>,
) -> HostResult<T> {
  let effects = ScopedEffects::install();
  let result = operation();
  if result.is_err()
    && !super::callback_failure::in_causal_invocation()
    && let Some(cause) = host.failure_cause()
    && let Some(failure) = cause.downcast_ref::<EvaluationFailure>()
    && failure.plain
  {
    stylex_macros::stylex_error::mark_panic_plain();
  }
  state.queue_evaluation_effects(effects.finish());
  if state.pending_effects_overflowed() {
    let _ = stylex_js::native::refuse_resource::<()>();
    if result.is_ok() {
      return Err(HostError {
        message: "Static evaluation exceeded its resource limit.".into(),
      });
    }
  }
  result
}
