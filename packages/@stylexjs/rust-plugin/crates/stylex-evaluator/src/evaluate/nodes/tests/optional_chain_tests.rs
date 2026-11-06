//! Tests optional-member reads, nullish bases and unsupported optional calls.

use crate::evaluate::source_evaluation::*;
use stylex_constants::constants::evaluation_errors::unsupported_expression;

#[test]
fn a_chain_reads_through_a_base_the_evaluator_holds_as_its_own_value() {
  assert_folds_to_number("['a', 'b']?.length", 2.0);
  assert_folds_to_string("['a', 'b']?.[1]", "b");
}

#[test]
fn a_chain_reads_through_an_object_base() {
  assert_folds_to_string("({ color: 'red' })?.color", "red");
  assert_folds_to_string("({ theme: { color: 'red' } })?.theme?.color", "red");
}

#[test]
fn a_method_called_through_the_question_mark_refuses() {
  for source in ["['a', 'b']?.join('-')", "'red'?.toUpperCase()"] {
    assert_deopt_reason_contains(source, &unsupported_expression("OptionalCallExpression"));
  }
}

/// Optional member reads extend Babel; optional calls remain unsupported (`lib/index.js:6942`).
#[test]
fn an_optional_call_refuses_whatever_its_callee() {
  assert_deopt_reason_contains(
    "((color) => color)?.('red')",
    &unsupported_expression("OptionalCallExpression"),
  );
}

#[test]
fn a_nullish_base_stops_the_chain() {
  for source in [
    "null?.color",
    "undefined?.color",
    "null?.()",
    "undefined?.()",
    "null?.['color']",
  ] {
    assert_deopts(source);
  }
}
