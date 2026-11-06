//! Tests for the ValueWithDefault enum's accessor methods.

use indexmap::IndexMap;

use crate::value_with_default::ValueWithDefault;

/// Verifies that `as_map()` returns Some for the Map variant.
#[test]
fn as_map_with_map_returns_some() {
  let mut inner = IndexMap::new();
  inner.insert(
    "key".to_string(),
    ValueWithDefault::String("val".to_string()),
  );
  let value = ValueWithDefault::Map(inner.clone());
  assert_eq!(value.as_map(), Some(&inner));
}

/// Verifies that `as_map()` returns None for the String variant.
#[test]
fn as_map_with_string_returns_none() {
  let value = ValueWithDefault::String("hello".to_string());
  assert_eq!(value.as_map(), None);
}

/// Verifies that `as_map()` returns None for the Number variant.
#[test]
fn as_map_with_number_returns_none() {
  let value = ValueWithDefault::Number(42.0);
  assert_eq!(value.as_map(), None);
}
