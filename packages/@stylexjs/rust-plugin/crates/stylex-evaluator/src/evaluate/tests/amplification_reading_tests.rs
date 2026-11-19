//! Tests receiver and count readings used by amplification bounds.

use super::source_evaluation::*;
use stylex_constants::constants::evaluation_errors::unsupported_expression;

const UNSUPPORTED_CALL: &str = "Unsupported expression: CallExpression";

#[test]
fn a_written_array_receiver_is_measured_by_its_elements() {
  assert_folds_to_string(
    "({ a: [1, 2] }).a.map((each) => each + 'px').join('-')",
    "1px-2px",
  );
  assert_folds_to_string(
    "({ a: ['aa', 'b'] }).a.map((each) => each.repeat(2)).join('-')",
    "aaaa-bb",
  );
}

#[test]
fn a_written_element_of_fixed_width_is_measured_as_it_is_written() {
  assert_folds_to_string(
    "({ a: [true, false] }).a.map((each) => each + '!').join('-')",
    "true!-false!",
  );
  assert_folds_to_string(
    "({ a: [null, undefined] }).a.map((each) => each + '!').join('-')",
    "null!-undefined!",
  );
  assert_folds_to_string(
    "({ a: [[1], [2]] }).a.map((each) => each + '!').join('-')",
    "1!-2!",
  );
}

#[test]
fn array_from_measures_a_written_array_the_same_way() {
  assert_folds_to_string(
    "Array.from(({ a: [1, 2] }).a, (each) => each + 'px').join('-')",
    "1px-2px",
  );
}

#[test]
fn a_string_source_is_counted_by_its_code_units() {
  assert_folds_to_string("Array.from('ab', (each) => each + '!').join('-')", "a!-b!");
}

#[test]
fn a_source_with_no_count_iterates_nothing() {
  assert_folds_to_strings("Array.from(({ a: 1 }).a, () => 'ab'.repeat(3))", &[]);
}

#[test]
fn array_from_with_nothing_to_iterate_refuses() {
  // Unlike Node/Babel's throw, a missing Array.from source returns a deopt.
  assert_deopt_reason_contains("Array.from()", UNSUPPORTED_CALL);
}

#[test]
fn a_count_written_as_a_product_is_worked_out() {
  assert_folds_to_string("'ab'.repeat(2 * 3)", "abababababab");
  assert_folds_to_string("'ab'.repeat((1 + 1) * 2)", "abababab");
}

#[test]
fn a_count_a_name_holds_is_worked_out() {
  assert_eq!(
    folded_in_a_module_binding("count", "3", "'ab'.repeat(count)"),
    "ababab"
  );
}

#[test]
fn a_count_a_name_holds_as_a_list_is_read_as_the_array_it_stands_for() {
  assert_eq!(
    folded_in_a_module_binding("count", "[3]", "'ab'.repeat(count)"),
    "ababab"
  );

  for empty in ["[]", "[1, 2]"] {
    assert_eq!(
      folded_in_a_module_binding("count", empty, "'ab'.repeat(count)"),
      ""
    );
  }
}

#[test]
fn a_receiver_a_name_holds_is_measured_like_the_literal() {
  assert_eq!(
    folded_in_a_module_binding("part", "'ab'", "part.repeat(2)"),
    "abab"
  );
}

#[test]
fn a_receiver_that_is_not_a_string_has_no_repeat() {
  assert_deopt_reason_contains("({ a: 1 }).a.repeat(3)", UNSUPPORTED_CALL);
}

#[test]
fn a_count_written_below_zero_is_left_to_the_language() {
  // Unlike Node/Babel's RangeError, a negative repeat count returns a deopt here.
  assert_deopt_reason_contains("'ab'.repeat(-1)", UNSUPPORTED_CALL);
}

#[test]
fn a_negative_leaf_folds_per_element_rather_than_stopping_the_call() {
  assert_folds_to_strings(
    "[1, 2].map((each) => 'ab'.repeat(each + ({ k: -1 }).k))",
    &["", "ab"],
  );
}

#[test]
fn a_count_a_callback_builds_is_worked_out_from_its_element() {
  assert_folds_to_string(
    "[1, 2].map((each) => 'x'.repeat(each * 2)).join('-')",
    "xx-xxxx",
  );
  assert_folds_to_string(
    "[1, 2].map((each) => 'x'.repeat(each + 1)).join('-')",
    "xx-xxx",
  );
}

#[test]
fn a_count_a_callback_builds_reads_a_leaf_the_module_holds() {
  assert_folds_to_string(
    "[1, 2].map((each) => 'x'.repeat(each + ({ k: 1 }).k)).join('-')",
    "xx-xxx",
  );
}

#[test]
#[should_panic(expected = "Unsupported expression: CallExpression")]
fn an_element_below_zero_refuses_the_count_it_sizes() {
  assert_folds_to_string("[-1, 2].map((each) => each + '!').join('-')", "-1!-2!");
  // Match PR #2008 at 6fc3a507: callback refusal throws a plain compiler Error without a frame.
  evaluate_source("[-1, 2].map((each) => 'x'.repeat(each * 2))");
}

#[test]
fn a_length_declared_before_another_key_is_still_read() {
  assert_folds_to_number("Array.from({ length: 2, a: 1 }).length", 2.0);
  assert_folds_to_number("Array.from({ length: 1, length: 2 }).length", 2.0);
}

#[test]
fn the_quoted_spelling_of_the_length_key_is_the_same_key() {
  assert_folds_to_number("Array.from({ 'length': 3 }).length", 3.0);
}

#[test]
fn a_key_that_is_not_length_declares_nothing() {
  assert_folds_to_number("Array.from({ len: 2000000 }).length", 0.0);
  assert_folds_to_number("Array.from({ 0: 2000000 }).length", 0.0);

  assert_deopt_reason_contains(
    "Array.from({ length: 2000000 })",
    "It declares a length of 2000000 elements",
  );
}

#[test]
fn a_length_the_language_rejects_declares_nothing() {
  assert_folds_to_number("Array.from({ length: -1 }).length", 0.0);
}

#[test]
fn a_receiver_the_evaluator_refuses_is_measured_as_nothing() {
  assert_deopt_reason_contains(
    "[1n].map((each) => each + '!')",
    &unsupported_expression("BigIntLiteral"),
  );
  assert_deopt_reason_contains(
    "Array.from([1n], (each) => each + '!')",
    &unsupported_expression("BigIntLiteral"),
  );
}

#[test]
fn a_count_that_resolves_to_a_function_has_no_number() {
  // Deliberately refuse an unreadable function count where Node uses repeat(NaN) and returns empty text.
  assert_deopt_reason_contains("'x'.repeat((() => 2))", UNSUPPORTED_CALL);
}

#[test]
#[should_panic(expected = "Unsupported expression: BigIntLiteral")]
fn a_count_leaf_nothing_can_answer_for_stops_the_arithmetic() {
  // Babel throws this compiler error from the executed callback; it is not a returned deopt.
  evaluate_source("[1, 2].map((each) => 'x'.repeat(each + 1n))");
}

#[test]
fn a_written_elements_magnitude_bounds_the_count_it_sizes() {
  assert_folds_to_string(
    "({ a: [2, 3] }).a.map((each) => 'x'.repeat(each)).join('-')",
    "xx-xxx",
  );
}

#[test]
fn one_nested_element_with_no_width_leaves_its_join_unread() {
  assert_folds_to_string(
    "({ a: [[{}], [1]] }).a.map((each) => 'x'.repeat(2)).join('-')",
    "xx-xx",
  );
}

#[test]
fn a_name_the_callback_binds_to_the_whole_receiver_counts_as_the_language_reads_it() {
  assert_folds_to_strings("['a'].map((each, index, all) => 'y'.repeat(all))", &[""]);
}

#[test]
#[should_panic(expected = "Referenced constant is not defined.")]
fn a_name_nothing_answers_for_bounds_nothing() {
  // Babel throws at callback invocation (`utils/evaluate-path.js:511`).
  evaluate_source("['a'].map((each) => 'x'.repeat(missing))");
}

#[test]
fn a_defaulted_parameter_refuses_the_callback_before_any_length_is_read() {
  // Babel admits only plain identifier parameters (`utils/evaluate-path.js:499`).
  assert_deopt_reason_contains(
    "['a'].map((each = 1) => Array(3).fill(each))",
    &unsupported_expression("ArrowFunctionExpression"),
  );
}
