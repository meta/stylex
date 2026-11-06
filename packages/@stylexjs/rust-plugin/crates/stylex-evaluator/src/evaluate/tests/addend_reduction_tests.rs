//! What `+` reads off each of its sides before it decides what it is.
//!
//! `+` is the one operator that applies `ToPrimitive` with no hint: an object
//! answers through its own `valueOf` first and its own `toString` second, and
//! the result of that reduction is what settles addition against
//! concatenation. Decided on the unreduced sides instead, an object with a
//! `valueOf` fell to the string path and `({ valueOf: () => 2 }) + 1` wrote
//! `[object Object]1` where the language and the reference implementation both
//! answer `3`.

use super::source_evaluation::*;

/// An object whose own `valueOf` answers a number is added, not concatenated.
#[test]
fn an_object_answering_a_number_is_added() {
  assert_folds_to_number("({ valueOf: () => 2 }) + 1", 3.0);
  assert_folds_to_number("1 + ({ valueOf: () => 2 })", 3.0);
  assert_folds_to_number("({ valueOf: () => 2 }) + ({ valueOf: () => 3 })", 5.0);
}

/// An object whose own `valueOf` answers a string concatenates, because the
/// reduction is what decides and the reduced value is a string.
#[test]
fn an_object_answering_a_string_concatenates() {
  assert_folds_to_string("({ valueOf: () => '2' }) + 1", "21");
  assert_folds_to_string("1 + ({ valueOf: () => '2' })", "12");
}

/// `valueOf` is asked before `toString`, which is the whole of what the default
/// hint means. Both spellings were already right when only one method was
/// there; the pair is what tells the two orders apart.
#[test]
fn value_of_is_asked_before_to_string() {
  assert_folds_to_number("({ valueOf: () => 2, toString: () => '9' }) + 1", 3.0);
  assert_folds_to_string("({ toString: () => '5' }) + 1", "51");
}

/// An object that keeps the `Object.prototype` pair has no own reduction, so it
/// reaches the string path and writes the default text -- which is what the
/// language concatenates too.
#[test]
fn an_object_with_no_own_conversion_concatenates_its_default_text() {
  assert_folds_to_string("({}) + 1", "[object Object]1");
  assert_folds_to_string("({ a: 1 }) + 1", "[object Object]1");
}

/// An array owns neither method, so it reduces to its join -- which is the
/// string path's reading of it as well.
#[test]
fn an_array_concatenates_as_its_join() {
  assert_folds_to_string("[1, 2] + 1", "1,21");
  assert_folds_to_string("[] + 1", "1");
}

/// A method answering an object has answered no primitive, so the language moves
/// on to the other method -- here the inherited `toString`, which answers the
/// default text. The reduction hands the object on rather than inventing a value
/// for it, and the string path writes that same text.
#[test]
fn an_object_whose_conversion_answers_an_object_writes_the_default_text() {
  assert_folds_to_string("({ valueOf: () => ({}) }) + 1", "[object Object]1");
}

/// The reduction belongs to `+` alone. `===` compares the two values as they
/// stand, so an object it cannot compare by reference has no answer here and
/// the walk deopts -- rather than being handed the `true` a reduced side would
/// have produced, which is not what the language answers.
#[test]
fn strict_equality_does_not_reduce_its_sides() {
  assert_deopts("(({ valueOf: () => 1 }) === 1) ? 'red' : 'blue'");
}
