//! Tests for the object helpers the theme and variable transforms build on.

use indexmap::IndexMap;
use stylex_state::state_manager::StateManager;
use stylex_structures::{order_pair::OrderPair, pair::Pair, raw_value::TRawValue};

use crate::shared::utils::object::{
  obj_entries, obj_from_entries, obj_map_keys_and_transform_values, preprocess_object_properties,
};
use crate::tests::support::expr;

#[test]
fn reads_the_entries_of_an_object_literal() {
  let entries = obj_entries(&expr("{ color: 'red', size: 1 }"));

  assert_eq!(entries.len(), 2);
}

#[test]
fn reads_no_entry_from_an_empty_object_literal() {
  assert!(obj_entries(&expr("{}")).is_empty());
}

/// Only an object literal holds entries to read. Anything else is a fault in
/// the caller, so the call is refused rather than answered with nothing.
#[test]
#[should_panic(expected = "Object expected")]
fn refuses_to_read_the_entries_of_something_that_is_not_an_object() {
  obj_entries(&expr("[1, 2]"));
}

#[test]
fn builds_a_map_from_the_pairs_that_hold_a_value() {
  let entries = vec![
    OrderPair("color".into(), Some("red".into())),
    OrderPair("size".into(), Some("1px".into())),
  ];

  let result = obj_from_entries(&entries);

  assert_eq!(
    result.keys().cloned().collect::<Vec<_>>(),
    ["color", "size"]
  );
  assert_eq!(result["color"], "red");
}

/// A repeated key keeps the place the first of the two gave it, and holds the
/// value of the last.
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

#[test]
fn builds_an_empty_map_from_no_pair() {
  assert!(obj_from_entries(&[]).is_empty());
}

/// A pair holding no value cannot become an entry, and dropping it silently
/// would leave the caller with a map short of a key it wrote.
#[test]
#[should_panic(expected = "Value is not a string")]
fn refuses_a_pair_that_holds_no_value() {
  obj_from_entries(&[OrderPair("color".into(), None)]);
}

/// The key is dashed before the value is read, because the dashed name is the
/// one that decides whether a plain number gets a unit.
#[test]
fn dashes_the_key_before_it_transforms_the_value() {
  let mut state = StateManager::default();
  let mut entries: IndexMap<String, TRawValue> = IndexMap::new();

  entries.insert("marginTop".to_owned(), TRawValue::Number(10.0));

  let result = obj_map_keys_and_transform_values(&entries, &mut state, |key| {
    key.replace("marginTop", "margin-top")
  });

  assert_eq!(result, vec![Pair::new("margin-top", "10px")]);
}

/// A property that takes no unit keeps the number it was written with.
#[test]
fn keeps_a_unitless_value_as_it_was_written() {
  let mut state = StateManager::default();
  let mut entries: IndexMap<String, TRawValue> = IndexMap::new();

  entries.insert("opacity".to_owned(), TRawValue::Number(0.5));
  entries.insert("zIndex".to_owned(), TRawValue::Number(2.0));

  let result = obj_map_keys_and_transform_values(&entries, &mut state, |key| key.to_owned());

  assert_eq!(
    result,
    vec![Pair::new("opacity", ".5"), Pair::new("zIndex", "2")]
  );
}

#[test]
fn maps_no_key_of_an_empty_entry_set() {
  let mut state = StateManager::default();

  assert!(
    obj_map_keys_and_transform_values(&IndexMap::new(), &mut state, |key| key.to_owned())
      .is_empty()
  );
}

/// Two keys that map to the same name are one declaration. The first place is
/// kept, and the value written last wins.
#[test]
fn folds_two_keys_that_map_to_one_name_into_one_pair() {
  let mut state = StateManager::default();
  let mut entries: IndexMap<String, TRawValue> = IndexMap::new();

  entries.insert("marginTop".to_owned(), TRawValue::String("1px".to_owned()));
  entries.insert("zIndex".to_owned(), TRawValue::Number(2.0));
  entries.insert("margin-top".to_owned(), TRawValue::String("3px".to_owned()));

  let result = obj_map_keys_and_transform_values(&entries, &mut state, |key| {
    key.replace("marginTop", "margin-top")
  });

  assert_eq!(
    result,
    vec![Pair::new("margin-top", "3px"), Pair::new("zIndex", "2")]
  );
}

/// An alias is written under the property it stands for. Only the properties
/// that carry a value survive, because the rest declare nothing.
#[test]
fn writes_an_alias_under_the_property_it_stands_for() {
  let mut state = StateManager::default();

  let result = preprocess_object_properties(&expr("{ blockSize: '10px' }"), &mut state);

  assert_eq!(result.keys().cloned().collect::<Vec<_>>(), ["height"]);
  assert_eq!(result["height"], "10px");
}

/// A shorthand keeps the value on its own key and leaves the properties it
/// reaches empty, so only the shorthand itself reaches the answer.
#[test]
fn expands_a_shorthand_and_keeps_the_keys_that_carry_a_value() {
  let mut state = StateManager::default();

  let result = preprocess_object_properties(&expr("{ margin: '10px' }"), &mut state);

  assert_eq!(result.keys().cloned().collect::<Vec<_>>(), ["margin"]);
  assert_eq!(result["margin"], "10px");
}

/// A number stays a number through the expansion, so the unit is added by the
/// property that reads it rather than by the shorthand.
#[test]
fn a_number_reaches_the_expansion_as_a_number() {
  let mut state = StateManager::default();

  let result = preprocess_object_properties(&expr("{ margin: 10 }"), &mut state);

  assert_eq!(result["margin"].as_number(), Some(10.0));
}

#[test]
fn keeps_a_property_that_stands_for_itself() {
  let mut state = StateManager::default();

  let result = preprocess_object_properties(&expr("{ color: 'red' }"), &mut state);

  assert_eq!(result.keys().cloned().collect::<Vec<_>>(), ["color"]);
  assert_eq!(result["color"], "red");
}

#[test]
fn preprocesses_an_empty_object_to_nothing() {
  let mut state = StateManager::default();

  assert!(preprocess_object_properties(&expr("{}"), &mut state).is_empty());
}
