//! Tests evaluator refusal logging with a thread-local capturing logger.

use super::source_evaluation::*;
use crate::tests::capturing_logger::logged_at;

use log::Level;

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
fn a_key_that_is_not_an_expression_writes_what_was_read_and_what_came_back() {
  let messages = logged_at(Level::Debug, || evaluate_source("({ a: 1 })[['a']]"));

  assert_logged(&messages, "Property not found for expression: Object(");
  assert_logged(&messages, "Evaluation result: Vec(");
  assert_logged(&messages, "Original property: Computed(");
}
