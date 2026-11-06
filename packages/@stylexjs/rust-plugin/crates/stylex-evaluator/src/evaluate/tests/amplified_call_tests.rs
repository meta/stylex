//! A call that would build more than it was written with.
//!
//! `'ab'.repeat(1000)` is three characters of source and two thousand of
//! result, and inside a callback it is that again once per element. So the
//! fold works out what a call would come to before it runs one, in the two
//! units it spends -- characters of string and array entries -- and refuses a
//! product it cannot bound.
//!
//! What each case here asserts is that the arithmetic *admits* the calls a
//! stylesheet is actually written with. The refusals have their own suite; what
//! is easy to lose is the other side, where a bound the walk could not read
//! turns an ordinary declaration into one that falls to the runtime.

use super::source_evaluation::*;

/// A string amplifier inside a callback is measured against the receiver's own
/// count, so a small repetition per element still folds.
#[test]
fn a_string_amplifier_inside_a_callback_folds() {
  assert_folds_to_string("['a'].map(() => 'ab'.repeat(3)).join('-')", "ababab");
  assert_folds_to_string(
    "['a', 'b'].map((part) => part.repeat(2)).join('-')",
    "aa-bb",
  );
}

/// A count written as an expression is worked out rather than refused: the rule
/// is arithmetic, not syntax, so a product of two literals bounds the call as
/// well as a literal does.
#[test]
fn a_count_written_as_an_expression_is_worked_out() {
  assert_folds_to_string("'ab'.repeat(2 * 2)", "abababab");
  assert_folds_to_string("'ab'.repeat(1 + 1)", "abab");
}

/// An array amplifier is measured in entries rather than characters, and folds
/// on the same terms -- inside a callback and out.
#[test]
fn an_array_amplifier_folds_in_entries() {
  assert_folds_to_string("['a'].map(() => [1, 2].fill(0)).flat().join('-')", "0-0");
  assert_folds_to_string(
    "['a'].map(() => Array(3).fill('x').join('')).join('-')",
    "xxx",
  );
}

/// A length declared as a property rather than written as an argument is read
/// the same way, which is what lets `Array.from({ length: n })` fold.
#[test]
fn a_declared_length_bounds_the_call_it_sizes() {
  assert_folds_to_number("Array.from({ length: 3 }).length", 3.0);
  assert_folds_to_string("Array.from({ length: 2 }, () => 'x').join('')", "xx");
}

/// A number is measured by how wide it is written, so a call sized by one is
/// bounded by its magnitude rather than by the digits of the source.
#[test]
fn a_numeric_element_is_measured_by_what_it_writes() {
  assert_folds_to_string(
    "[1, 2].map((each) => each.toFixed(2)).join('-')",
    "1.00-2.00",
  );
  assert_folds_to_string("[10, 200].map((each) => each + '').join('-')", "10-200");
}

/// The values with a fixed written width -- a boolean, `null`, `undefined` --
/// are measured by the text the language writes for them.
#[test]
fn the_values_with_a_fixed_width_are_measured_as_they_are_written() {
  assert_folds_to_string("[true, false].join('-')", "true-false");
  assert_folds_to_string(
    "[null, undefined].map((each) => each + '').join('-')",
    "null-undefined",
  );
  assert_folds_to_string("[['a'], ['b']].map((each) => each + '').join('-')", "a-b");
}
