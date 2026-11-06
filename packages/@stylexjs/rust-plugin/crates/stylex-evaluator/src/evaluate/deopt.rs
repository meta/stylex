use super::*;

use stylex_diagnostics::code_frame::frame_declaration_of;

pub(crate) fn deopt(
  path: &Expr,
  state: &mut EvaluationState,
  reason: &str,
) -> Option<EvaluateResultValue> {
  if state.confident {
    state.confident = false;
    state.deopt_path = Some(path.clone());
    state.deopt_reason = Some(reason.to_string());
  }

  None
}

pub(crate) fn deopt_at_declaration(
  path: &Expr,
  name: &Atom,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  reason: &str,
) -> Option<EvaluateResultValue> {
  if state.confident {
    frame_declaration_of(name, path, traversal_state);
  }

  deopt(path, state, reason)
}
