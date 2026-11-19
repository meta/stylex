//! Tests amplification bounds for strings, arrays and callback results.

use super::source_evaluation::*;

#[test]
fn a_string_amplifier_inside_a_callback_folds() {
  assert_folds_to_string("['a'].map(() => 'ab'.repeat(3)).join('-')", "ababab");
  assert_folds_to_string(
    "['a', 'b'].map((part) => part.repeat(2)).join('-')",
    "aa-bb",
  );
}

#[test]
fn a_count_written_as_an_expression_is_worked_out() {
  assert_folds_to_string("'ab'.repeat(2 * 2)", "abababab");
  assert_folds_to_string("'ab'.repeat(1 + 1)", "abab");
}

/// Mutating array methods deliberately refuse even fresh receivers that Babel folds.
#[test]
#[should_panic(expected = "Unsupported expression: CallExpression")]
fn an_array_amplifier_refuses_in_entries() {
  evaluate_source("['a'].map(() => [1, 2].fill(0)).flat().join('-')");
}

#[test]
#[should_panic(expected = "Unsupported expression: CallExpression")]
fn an_amplified_array_fill_refuses_in_entries() {
  evaluate_source("['a'].map(() => Array(3).fill('x').join('')).join('-')");
}

#[test]
fn a_declared_length_bounds_the_call_it_sizes() {
  assert_folds_to_number("Array.from({ length: 3 }).length", 3.0);
  assert_folds_to_string("Array.from({ length: 2 }, () => 'x').join('')", "xx");
}

#[test]
fn a_numeric_element_is_measured_by_what_it_writes() {
  assert_folds_to_string(
    "[1, 2].map((each) => each.toFixed(2)).join('-')",
    "1.00-2.00",
  );
  assert_folds_to_string("[10, 200].map((each) => each + '').join('-')", "10-200");
}

#[test]
fn the_values_with_a_fixed_width_are_measured_as_they_are_written() {
  assert_folds_to_string("[true, false].join('-')", "true-false");
  assert_folds_to_string(
    "[null, undefined].map((each) => each + '').join('-')",
    "null-undefined",
  );
  assert_folds_to_string("[['a'], ['b']].map((each) => each + '').join('-')", "a-b");
}
