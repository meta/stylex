//! Tests evaluator refusal logging with a thread-local capturing logger.

use super::source_evaluation::*;
use crate::tests::capturing_logger::logged_at;

use std::rc::Rc;

use log::Level;
use stylex_state::{
  functions::{FunctionMap, FunctionType},
  theme_ref::ThemeRef,
};

#[track_caller]
fn assert_logged(messages: &[String], fragment: &str) {
  assert!(
    messages.iter().any(|message| message.contains(fragment)),
    "expected a message saying `{}`, and the log holds {:?}",
    fragment,
    messages
  );
}

#[test]
fn an_unsupported_expression_names_its_kind_in_the_log() {
  let warned = logged_at(Level::Warn, || evaluate_source("class {}"));

  assert_logged(&warned, "Unsupported type of expression: ClassExpression");
  assert_logged(&warned, "please recompile using debug mode");

  let debugged = logged_at(Level::Debug, || evaluate_source("class {}"));

  assert_logged(&debugged, "Unsupported type of expression: Class");
}

#[test]
fn a_binary_expression_that_folds_to_nothing_writes_both_failures() {
  let messages = logged_at(Level::Debug, || evaluate_source("(() => 1) + 1"));

  assert_logged(&messages, "Binary expression to number error");
  assert_logged(&messages, "Binary expression to string error");
}

#[test]
fn a_group_used_as_an_object_key_is_reported_at_warn() {
  let messages = logged_at(Level::Warn, || {
    evaluated_against(&a_group_named("colors"), "({ a: 1 })[colors]")
  });

  assert_logged(&messages, "used as an object key");
  assert_logged(&messages, "code minification");
}

#[test]
fn a_group_used_as_an_object_key_writes_the_object_the_group_and_the_key() {
  let messages = logged_at(Level::Debug, || {
    evaluated_against(&a_group_named("colors"), "({ a: 1 })[colors]")
  });

  assert_logged(&messages, "Evaluating member access on object:");
  assert_logged(&messages, "Object expression:");
  assert_logged(&messages, "Theme reference:");
  assert_logged(&messages, "Original property:");
}

#[test]
fn a_key_that_is_not_an_expression_writes_what_was_read_and_what_came_back() {
  let messages = logged_at(Level::Debug, || evaluate_source("({ a: 1 })[['a']]"));

  assert_logged(&messages, "Property not found for expression: Object(");
  assert_logged(&messages, "Evaluation result: Vec(");
  assert_logged(&messages, "Original property: Computed(");
}

fn a_group_named(name: &str) -> FunctionMap {
  let theme = ThemeRef::new("vars.stylex.js", "vars", "x");
  let mut fns = FunctionMap::default();

  fns.identifiers.insert(
    name.into(),
    Box::new(folded_entry(
      FunctionType::ThemeRefMapper(Rc::new(move || theme.clone())),
      false,
    )),
  );

  fns
}
