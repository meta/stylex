//! Object helpers the theme and variable transforms build on.

use stylex_structures::order_pair::OrderPair;

use crate::shared::utils::object::{obj_entries, obj_from_entries};
use crate::tests::support::expr;

/// A non-object is a caller fault, refused rather than answered with nothing.
#[test]
#[should_panic(expected = "Object expected")]
fn refuses_to_read_the_entries_of_something_that_is_not_an_object() {
  obj_entries(&expr("[1, 2]"));
}

#[test]
fn a_repeated_pair_keeps_its_first_place() {
  let entries = vec![
    OrderPair("color".into(), Some("red".into())),
    OrderPair("size".into(), Some("1px".into())),
    OrderPair("color".into(), Some("blue".into())),
  ];

  let result = obj_from_entries(&entries);

  assert_eq!(
    result.keys().cloned().collect::<Vec<_>>(),
    ["color", "size"]
  );
  assert_eq!(result["color"], "blue");
}

/// Dropping it would leave the caller's map short of a key.
#[test]
#[should_panic(expected = "Value is not a string")]
fn refuses_a_pair_that_holds_no_value() {
  obj_from_entries(&[OrderPair("color".into(), None)]);
}
