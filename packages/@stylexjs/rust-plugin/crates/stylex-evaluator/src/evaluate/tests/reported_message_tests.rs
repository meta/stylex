//! What the evaluator writes to the log when it declines to fold something.
//!
//! A `log` macro reads its arguments only while `log::max_level` admits their
//! level, so every message below is unexecuted code in an ordinary build,
//! where nothing installs a logger and the level stays `Off`. This test binary
//! opens the level for the whole process, which is what makes the arguments
//! run; every other suite here then runs them without reading them, asserting
//! the refusal and not the sentence the author is meant to read beside it.
//! These cases run the same shapes and read the messages back.
//!
//! Each case names the words it expects rather than only counting the
//! messages. A message that said nothing about the expression would satisfy a
//! count and tell an author nothing, which is the mistake worth catching here.

use super::source_evaluation::*;
use crate::tests::capturing_logger::logged_at;

use std::rc::Rc;

use log::Level;
use stylex_state::{
  functions::{FunctionMap, FunctionType},
  theme_ref::ThemeRef,
};

/// Asserts one of `messages` holds `fragment`, and says what was written where
/// it does not.
#[track_caller]
fn assert_logged(messages: &[String], fragment: &str) {
  assert!(
    messages.iter().any(|message| message.contains(fragment)),
    "expected a message saying `{}`, and the log holds {:?}",
    fragment,
    messages
  );
}

/// An expression kind the dispatch has no arm for is reported twice: once in
/// the words an author reads, and once as the tree itself for a debug build.
///
/// The kind is read once and both the sentence and the message use that
/// reading, so a case that only asserted the refusal would leave the second
/// message free to name a different expression.
#[test]
fn an_unsupported_expression_names_its_kind_in_the_log() {
  let warned = logged_at(Level::Warn, || evaluate_source("class {}"));

  assert_logged(&warned, "Unsupported type of expression: ClassExpression");
  assert_logged(&warned, "please recompile using debug mode");

  let debugged = logged_at(Level::Debug, || evaluate_source("class {}"));

  assert_logged(&debugged, "Unsupported type of expression: Class");
}

/// A binary expression that reads as neither a number nor a string writes both
/// failures, so a reader can tell which coercion was asked for first.
///
/// The two are different sentences over the same expression, and the case names
/// both: one message alone cannot say whether the string path was even tried.
#[test]
fn a_binary_expression_that_folds_to_nothing_writes_both_failures() {
  let messages = logged_at(Level::Debug, || evaluate_source("(() => 1) + 1"));

  assert_logged(&messages, "Binary expression to number error");
  assert_logged(&messages, "Binary expression to string error");
}

/// A theme group standing where an object key was expected is reported at
/// `warn`, because it is a shape an author can neither write on purpose nor
/// read out of the refusal alone.
#[test]
fn a_group_used_as_an_object_key_is_reported_at_warn() {
  let messages = logged_at(Level::Warn, || {
    evaluated_against(&a_group_named("colors"), "({ a: 1 })[colors]")
  });

  assert_logged(&messages, "used as an object key");
  assert_logged(&messages, "code minification");
}

/// The same read at `debug` writes the three things a report of it needs: the
/// object, the group and the key as it was written.
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

/// A computed key that folded to something other than an expression writes the
/// receiver, the answer and the key as it was written.
///
/// An array is such a key: it folds to a list rather than to an expression, and
/// a list is not a name an object carries. The refusal names only the rule, so
/// these three messages are the whole record of which read stopped and on what.
#[test]
fn a_key_that_is_not_an_expression_writes_what_was_read_and_what_came_back() {
  let messages = logged_at(Level::Debug, || evaluate_source("({ a: 1 })[['a']]"));

  assert_logged(&messages, "Property not found for expression: Object(");
  assert_logged(&messages, "Evaluation result: Vec(");
  assert_logged(&messages, "Original property: Computed(");
}

/// One name bound to a theme group, which is how a group reaches an expression
/// at all — no source spells one, and the binding is the compiler's own.
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
