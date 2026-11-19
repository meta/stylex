//! Tests `ToPrimitive` ordering and addition versus concatenation.

use super::source_evaluation::*;

#[test]
fn an_object_answering_a_number_is_added() {
  assert_folds_to_number("({ valueOf: () => 2 }) + 1", 3.0);
  assert_folds_to_number("1 + ({ valueOf: () => 2 })", 3.0);
  assert_folds_to_number("({ valueOf: () => 2 }) + ({ valueOf: () => 3 })", 5.0);
}

#[test]
fn an_object_answering_a_string_concatenates() {
  assert_folds_to_string("({ valueOf: () => '2' }) + 1", "21");
  assert_folds_to_string("1 + ({ valueOf: () => '2' })", "12");
}

#[test]
fn value_of_is_asked_before_to_string() {
  assert_folds_to_number("({ valueOf: () => 2, toString: () => '9' }) + 1", 3.0);
  assert_folds_to_string("({ toString: () => '5' }) + 1", "51");
}

#[test]
fn an_object_with_no_own_conversion_concatenates_its_default_text() {
  assert_folds_to_string("({}) + 1", "[object Object]1");
  assert_folds_to_string("({ a: 1 }) + 1", "[object Object]1");
}

#[test]
fn an_array_concatenates_as_its_join() {
  assert_folds_to_string("[1, 2] + 1", "1,21");
  assert_folds_to_string("[] + 1", "1");
}

#[test]
fn an_object_whose_conversion_answers_an_object_writes_the_default_text() {
  assert_folds_to_string("({ valueOf: () => ({}) }) + 1", "[object Object]1");
}

#[test]
fn strict_equality_does_not_reduce_its_sides() {
  assert_folds_to_string("(({ valueOf: () => 1 }) === 1) ? 'red' : 'blue'", "blue");
}
