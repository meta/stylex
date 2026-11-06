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

/// The same refusal, reported against the declaration of `name` rather than
/// against the expression it was raised on.
///
/// Every step of the reference chain but the last deopts on `binding.path` —
/// the declaration — and only the tail deopts on the reference
/// (`utils/evaluate-path.js:626,647,653,657,661,665,673` against `:687`, 0.19.0).
/// The declaration is the line the author has to go and change, so that is the
/// line the code frame names on both sides.
///
/// The name is recorded rather than the declaration's span, because a span from
/// this compiler's parse means nothing in the code frame's own source map;
/// `stylex_diagnostics` turns the name back into a position in the
/// module the frame reads. Only a refusal that takes effect records one, so the
/// first refusal decides the position exactly as it decides the message.
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
