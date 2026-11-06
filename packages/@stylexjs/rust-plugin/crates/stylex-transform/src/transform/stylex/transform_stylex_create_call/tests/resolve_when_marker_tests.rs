//! How `stylex.when.*` reads its second-argument marker. Unit tests, because the warning's
//! words are built only when a logger admits the level, and the integration binary admits none.

use log::Level;
use stylex_state::{evaluate_result_value::EvaluateResultValue, state_manager::StateManager};
use stylex_structures::stylex_options::StyleXOptions;

use super::resolve_when_marker;
use crate::tests::capturing_logger::logged_at;
use crate::tests::support::expr;

fn state() -> StateManager {
  StateManager::new(StyleXOptions::default())
}

fn marker(code: &str) -> EvaluateResultValue {
  EvaluateResultValue::Expr(expr(code))
}

#[test]
fn a_class_name_resolves_without_a_word() {
  let state = state();
  let marker = marker("'the-marker'");

  let logged = logged_at(Level::Warn, || {
    let resolved = resolve_when_marker("ancestor", Some(&marker), &state);

    assert_eq!(resolved.as_str_value(), Some("the-marker"));
  });

  assert!(
    logged.is_empty(),
    "a class name is a marker the compiler can read, so it should say nothing: {logged:?}"
  );
}

/// Read through its class key.
#[test]
fn a_compiled_marker_resolves_without_a_word() {
  let state = state();
  let marker = marker("{ 'x1abc': 'x1abc', $$css: true }");

  let logged = logged_at(Level::Warn, || {
    let resolved = resolve_when_marker("descendant", Some(&marker), &state);

    assert_eq!(resolved.first_css_key(), Some("x1abc"));
  });

  assert!(
    logged.is_empty(),
    "a compiled marker is one the compiler can read, so it should say nothing: {logged:?}"
  );
}

/// Handed on rather than refused, as the reference implementation does, so the warning
/// naming the function and the fallback is the author's only clue.
#[test]
fn a_marker_the_compiler_cannot_read_is_reported_by_name() {
  let state = state();
  let marker = marker("42");

  let logged = logged_at(Level::Warn, || {
    resolve_when_marker("siblingAfter", Some(&marker), &state);
  });

  assert_eq!(logged.len(), 1, "expected one report, got {logged:?}");
  assert!(
    logged[0].contains("stylex.when siblingAfter"),
    "the report should name the function that read the marker: {}",
    logged[0]
  );
  assert!(
    logged[0].contains("default-marker"),
    "the report should name the fallback it took: {}",
    logged[0]
  );
}

/// Leaving the argument out is how an author asks for the configured default.
#[test]
fn an_absent_marker_takes_the_options_without_a_word() {
  let state = state();

  let logged = logged_at(Level::Warn, || {
    resolve_when_marker("anySibling", None, &state);
  });

  assert!(
    logged.is_empty(),
    "an absent marker is not a mistake, so it should say nothing: {logged:?}"
  );
}

#[test]
fn a_null_marker_takes_the_options_without_a_word() {
  let state = state();
  let marker = marker("null");

  let logged = logged_at(Level::Warn, || {
    resolve_when_marker("ancestor", Some(&marker), &state);
  });

  assert!(
    logged.is_empty(),
    "`null` asks for the configured default, so it should say nothing: {logged:?}"
  );
}
