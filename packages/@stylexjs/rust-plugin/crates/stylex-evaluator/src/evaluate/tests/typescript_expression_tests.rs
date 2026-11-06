//! What the evaluator does with the six expressions only a type system writes.
//!
//! `as`, `as const`, `satisfies`, `!`, `<T>x` and `f<T>` are erased before a
//! browser ever sees them, so each one has to fold to exactly what the
//! expression inside it folds to. A wrapper that folded to something else --
//! or that refused where the bare expression folds -- would make a declaration
//! depend on a type annotation, which is the one thing an annotation must
//! never do.
//!
//! Written in the TypeScript grammar rather than the one the other suites
//! read, because these six have no other spelling.

use super::source_evaluation::{
  assert_ts_deopt_reason_contains, assert_ts_folds_to_string, ts_folded_in_a_module_binding,
};
use stylex_constants::constants::evaluation_errors::UNDEFINED_CONST;

#[test]
fn an_as_assertion_folds_to_what_it_wraps() {
  assert_ts_folds_to_string("'red' as string", "red");
}

#[test]
fn a_const_assertion_folds_to_what_it_wraps() {
  assert_ts_folds_to_string("'red' as const", "red");
}

#[test]
fn a_satisfies_assertion_folds_to_what_it_wraps() {
  assert_ts_folds_to_string("'red' satisfies string", "red");
}

#[test]
fn a_non_null_assertion_folds_to_what_it_wraps() {
  assert_ts_folds_to_string("'red'!", "red");
}

#[test]
fn an_angle_bracket_assertion_folds_to_what_it_wraps() {
  assert_ts_folds_to_string("<string>'red'", "red");
}

/// Stacked wrappers are the shape a real file writes, and each layer has to
/// unwrap the one below it rather than only the outermost one being read.
#[test]
fn stacked_assertions_fold_to_what_the_innermost_one_wraps() {
  assert_ts_folds_to_string("(('red' as const) satisfies string)!", "red");
}

/// A type argument list is the sixth wrapper, and the only one whose inner
/// expression has to be a reference rather than any expression. It reaches the
/// same reading as the other five: the annotation is dropped and the name
/// behind it resolves to what the module bound it to.
#[test]
fn a_type_argument_list_folds_to_what_the_name_it_annotates_holds() {
  assert_eq!(
    ts_folded_in_a_module_binding("color", "'red'", "color<string>"),
    "red"
  );
}

/// A wrapper carries no value of its own, so a wrapped expression that cannot
/// fold refuses for the reason the expression inside it refuses -- not for the
/// annotation, which the author cannot usefully change. Both the wrapper that
/// takes any expression and the one that takes only a reference say so.
#[test]
fn a_wrapped_expression_that_cannot_fold_refuses_for_its_own_reason() {
  assert_ts_deopt_reason_contains("(unknownName as string)", UNDEFINED_CONST);
  assert_ts_deopt_reason_contains("unknownName<string>", UNDEFINED_CONST);
}
