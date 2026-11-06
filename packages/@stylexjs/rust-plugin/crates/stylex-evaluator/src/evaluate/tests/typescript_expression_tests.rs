//! Tests erased TypeScript wrappers and propagation of their inner values or refusals.

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

#[test]
fn stacked_assertions_fold_to_what_the_innermost_one_wraps() {
  assert_ts_folds_to_string("(('red' as const) satisfies string)!", "red");
}

#[test]
fn a_type_argument_list_folds_to_what_the_name_it_annotates_holds() {
  assert_eq!(
    ts_folded_in_a_module_binding("color", "'red'", "color<string>"),
    "red"
  );
}

#[test]
fn a_wrapped_expression_that_cannot_fold_refuses_for_its_own_reason() {
  assert_ts_deopt_reason_contains("(unknownName as string)", UNDEFINED_CONST);
  assert_ts_deopt_reason_contains("unknownName<string>", UNDEFINED_CONST);
}
