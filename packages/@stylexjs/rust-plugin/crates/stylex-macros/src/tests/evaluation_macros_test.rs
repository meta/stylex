//! Tests for the evaluation macros, using plain-type stubs in place of the evaluator.
//!
//! Evaluation and traversal state are separate stubs because that is what tells the macros apart.

/// Stands in for the evaluation state a refusal is recorded on.
#[derive(Default)]
struct EvalState {
  reason: Option<String>,
  calls: usize,
}

/// Stands in for the state manager a code frame is built from.
#[derive(Default)]
struct TraversalState {
  reads: usize,
}

/// Stands in for the evaluator's `deopt`: records the refusal and returns `None`.
fn record_deopt(expr: &str, state: &mut EvalState, reason: &str) -> Option<String> {
  state.calls += 1;
  state.reason = Some(format!("{expr}: {reason}"));

  None
}

fn refuse(expr: &str, state: &mut EvalState) -> Option<u8> {
  deopt_unsupported!(record_deopt, expr, state, "no static value");
}

#[test]
fn deopt_unsupported_records_the_reason_and_returns_none() {
  let mut state = EvalState::default();

  assert_eq!(refuse("fn()", &mut state), None);
  assert_eq!(state.reason.as_deref(), Some("fn(): no static value"));
  assert_eq!(state.calls, 1);
}

fn refuse_before_the_tail(state: &mut EvalState) -> Option<u8> {
  for step in 0..3u8 {
    if step == 1 {
      deopt_unsupported!(record_deopt, "loop", state, "refused mid-loop");
    }
  }

  Some(42)
}

#[test]
fn deopt_unsupported_leaves_the_function_from_inside_a_loop() {
  let mut state = EvalState::default();

  assert_eq!(refuse_before_the_tail(&mut state), None);
  assert_eq!(state.calls, 1);
}

fn refuse_in_a_match_arm(expr: &str, state: &mut EvalState) -> Option<u8> {
  let value = match expr {
    "one" => 1,
    other => deopt_unsupported!(record_deopt, other, state, "unknown shape"),
  };

  Some(value)
}

#[test]
fn deopt_unsupported_holds_a_value_position() {
  let mut state = EvalState::default();

  assert_eq!(refuse_in_a_match_arm("one", &mut state), Some(1));
  assert_eq!(state.calls, 0);

  assert_eq!(refuse_in_a_match_arm("two", &mut state), None);
  assert_eq!(state.reason.as_deref(), Some("two: unknown shape"));
}

/// Stands in for `wrap_in_paren_ref`.
fn wrap(expr: &str) -> String {
  format!("({expr})")
}

/// Stands in for the reporter without panicking, so a test can read what the macro passed.
fn report(wrapped: &str, expr: &str, msg: &str, state: &mut TraversalState) -> String {
  state.reads += 1;

  format!("{wrapped}|{expr}|{msg}")
}

#[test]
fn stylex_panic_with_context_reports_both_forms_of_the_expression() {
  let mut traversal_state = TraversalState::default();

  let reported = stylex_panic_with_context!(
    wrap,
    report,
    "x + y",
    &mut traversal_state,
    "broken invariant"
  );

  assert_eq!(reported, "(x + y)|x + y|broken invariant");
  assert_eq!(traversal_state.reads, 1);
}

/// Stands in for the reporter that really does end the process.
fn report_and_panic(wrapped: &str, _expr: &str, msg: &str, _state: &mut TraversalState) -> ! {
  panic!("[StyleX] {msg} at {wrapped}")
}

#[test]
#[should_panic(expected = "[StyleX] broken invariant at (x + y)")]
fn stylex_panic_with_context_carries_a_diverging_reporter() {
  let mut traversal_state = TraversalState::default();

  let _unreachable: u8 = stylex_panic_with_context!(
    wrap,
    report_and_panic,
    "x + y",
    &mut traversal_state,
    "broken invariant"
  );
}
