//! Tests erased TypeScript wrappers and propagation of their inner values or refusals.

use super::source_evaluation::{assert_ts_deopt_reason_contains, assert_ts_folds_to_string};
use stylex_constants::constants::evaluation_errors::UNDEFINED_CONST;

#[test]
fn a_non_null_assertion_folds_to_what_it_wraps() {
  assert_ts_folds_to_string("'red'!", "red");
}

#[test]
fn an_angle_bracket_assertion_folds_to_what_it_wraps() {
  assert_ts_folds_to_string("<string>'red'", "red");
}

#[test]
fn a_wrapped_expression_that_cannot_fold_refuses_for_its_own_reason() {
  assert_ts_deopt_reason_contains("(unknownName as string)", UNDEFINED_CONST);
  assert_ts_deopt_reason_contains("unknownName<string>", UNDEFINED_CONST);
}
