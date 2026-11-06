// Tests extracted for base_types.rs behaviors and edge cases.
// Source: crates/stylex-css-parser/src/base_types.rs

use super::*;

#[test]
fn test_new_with_string() {
  let substr = SubString::new("hello");
  assert_eq!(substr.string, "hello");
  assert_eq!(substr.start_index, 0);
  assert_eq!(substr.end_index, 4); // length - 1
}

#[test]
fn test_new_with_empty_string() {
  let substr = SubString::new("");
  assert_eq!(substr.string, "");
  assert_eq!(substr.start_index, 0);
  assert_eq!(substr.end_index, 0);
}

#[test]
fn test_starts_with() {
  let substr = SubString::new("hello world");
  assert!(substr.starts_with("hello"));
  assert!(substr.starts_with("h"));
  assert!(substr.starts_with(""));
  assert!(!substr.starts_with("world"));
  assert!(!substr.starts_with("hello world!")); // longer than original
}

#[test]
fn test_first() {
  let substr = SubString::new("hello");
  assert_eq!(substr.first(), Some('h'));

  let empty_substr = SubString::new("");
  assert_eq!(empty_substr.first(), None);
}

#[test]
fn test_get() {
  let substr = SubString::new("hello");
  assert_eq!(substr.get(0), Some('h'));
  assert_eq!(substr.get(1), Some('e'));
  assert_eq!(substr.get(4), Some('o'));
  assert_eq!(substr.get(5), None); // out of bounds
}

#[test]
fn test_to_string() {
  let substr = SubString::new("hello");
  assert_eq!(substr.to_string(), "hello");

  let empty_substr = SubString::new("");
  assert_eq!(empty_substr.to_string(), "");
}

#[test]
fn test_is_empty() {
  let substr = SubString::new("hello");
  assert!(!substr.is_empty());

  let empty_substr = SubString::new("");
  assert!(!empty_substr.is_empty()); // Even empty string has start_index=0, end_index=0

  // Test with manually modified indices
  let mut modified_substr = SubString::new("hello");
  modified_substr.start_index = 3;
  modified_substr.end_index = 2; // start_index > end_index
  assert!(modified_substr.is_empty());
}

#[test]
fn test_unicode_support() {
  let substr = SubString::new("héllo 🌍");
  assert_eq!(substr.first(), Some('h'));
  assert_eq!(substr.get(1), Some('é'));
  assert!(substr.starts_with("hé"));
  assert_eq!(substr.to_string(), "héllo 🌍");
}
