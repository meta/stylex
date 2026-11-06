//! What a speculative read puts back.
//!
//! The guard reads names to decide whether an expression *could* fold, and a
//! refusal raised while it decides is about nothing yet. Everything such a read
//! disturbs therefore has to be put back — on the path where it refused just as
//! much as on the path where it answered, because the refusing path is the only
//! one where there is anything to put back and the one a `return` between the
//! halves would skip.

use super::*;

use stylex_ast::ast::convertors::create_string_expr;

const CALLERS_PATH: &str = "the caller's own path";
const CALLERS_REASON: &str = "the caller's own reason";

/// A caller mid-evaluation: carrying a deopt path and a reason of its own, which
/// is the state a guard's read is reached under.
fn reading_state() -> EvaluationState {
  EvaluationState {
    deopt_path: Some(create_string_expr(CALLERS_PATH)),
    deopt_reason: Some(CALLERS_REASON.to_string()),
    ..EvaluationState::new()
  }
}

/// One string value, which is all a read has to answer with for the assertions
/// here to be about the putting-back rather than about the value.
fn a_value() -> EvaluateResultValue {
  EvaluateResultValue::Expr(create_string_expr("red"))
}

#[track_caller]
fn assert_is_the_callers_state(state: &EvaluationState) {
  assert!(state.confident, "confidence is the caller's again");

  assert_eq!(
    state.deopt_reason.as_deref(),
    Some(CALLERS_REASON),
    "the reason is the caller's again"
  );

  let path = match &state.deopt_path {
    Some(Expr::Lit(Lit::Str(path))) => path.value.as_str(),
    other => panic!("expected the caller's path, got {:?}", other),
  };

  assert_eq!(path, Some(CALLERS_PATH), "the path is the caller's again");
}

/// A read that refused leaves nothing of its refusal behind, and answers with no
/// value — so a caller cannot mistake a refusal for something the module held.
#[test]
fn a_refused_read_restores_the_state_it_read_under() {
  let mut state = reading_state();
  let mut traversal_state = StateManager::default();

  let read = speculate(
    &mut state,
    &mut traversal_state,
    |state, traversal_state| {
      assert!(
        traversal_state.speculating,
        "the read runs marked as a speculation"
      );

      state.confident = false;
      state.deopt_path = Some(create_string_expr("the read's own path"));
      state.deopt_reason = Some("the read's own reason".to_string());

      Some(a_value())
    },
  );

  assert!(
    read.is_none(),
    "a read that lost confidence answers with no value"
  );

  assert_is_the_callers_state(&state);
  assert!(!traversal_state.speculating, "the mark is put back too");
}

/// The same putting-back where the read answered, so the two paths cannot come
/// to restore different things.
#[test]
fn a_read_that_answered_restores_the_same_state() {
  let mut state = reading_state();
  let mut traversal_state = StateManager::default();

  let read = speculate(&mut state, &mut traversal_state, |_, _| Some(a_value()));

  assert!(read.is_some(), "a confident read answers with its value");

  assert_is_the_callers_state(&state);
  assert!(!traversal_state.speculating);
}

/// A fold reached from inside another speculation stays inside it: the mark is
/// saved and restored rather than cleared, so the outer read's refusals are still
/// withheld after the inner one has finished.
#[test]
fn a_nested_read_leaves_the_outer_speculation_marked() {
  let mut state = EvaluationState::new();
  let mut traversal_state = StateManager::default();

  speculate(
    &mut state,
    &mut traversal_state,
    |state, traversal_state| {
      speculate(state, traversal_state, |_, traversal_state| {
        assert!(traversal_state.speculating);

        None
      });

      assert!(
        traversal_state.speculating,
        "the outer read is still a speculation"
      );

      None
    },
  );

  assert!(
    !traversal_state.speculating,
    "and the outermost one is put back"
  );
}
