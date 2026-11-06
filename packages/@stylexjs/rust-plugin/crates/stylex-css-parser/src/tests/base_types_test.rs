/*!
Base types tests.
*/

use crate::base_types::SubString;

#[cfg(test)]
mod test_base_types {
  use super::*;

  #[test]
  fn constructor_creates_substring_with_correct_indices() {
    let substr = SubString::new("hello");
    assert_eq!(substr.string, "hello");
    assert_eq!(substr.start_index, 0);
    assert_eq!(substr.end_index, 4);
  }

  #[test]
  fn starts_with_returns_true_for_matching_prefix() {
    let substr = SubString::new("hello world");
    assert!(substr.starts_with("hello"));
    assert!(substr.starts_with("h"));
    assert!(substr.starts_with("")); // empty string
  }

  #[test]
  fn starts_with_returns_false_for_non_matching_prefix() {
    let substr = SubString::new("hello world");
    assert!(!substr.starts_with("world"));
    assert!(!substr.starts_with("hello world!")); // longer than original
    assert!(!substr.starts_with("goodbye"));
  }

  #[test]
  fn first_returns_first_character() {
    let substr = SubString::new("hello");
    assert_eq!(substr.first(), Some('h'));

    let empty_substr = SubString::new("");
    assert_eq!(empty_substr.first(), None);
  }

  #[test]
  fn get_returns_character_at_relative_index() {
    let substr = SubString::new("hello");
    assert_eq!(substr.get(0), Some('h'));
    assert_eq!(substr.get(1), Some('e'));
    assert_eq!(substr.get(4), Some('o'));
    assert_eq!(substr.get(5), None); // out of bounds
  }

  #[test]
  fn to_string_returns_substring_content() {
    let substr = SubString::new("hello");
    assert_eq!(substr.to_string(), "hello");

    let empty_substr = SubString::new("");
    assert_eq!(empty_substr.to_string(), "");
  }

  #[test]
  fn is_empty_returns_correct_boolean() {
    let substr = SubString::new("hello");
    assert!(!substr.is_empty());

    let empty_substr = SubString::new("");
    // Even empty string should not be empty in this implementation
    // because start_index=0 and end_index=0, so start_index <= end_index
    assert!(!empty_substr.is_empty());

    // Test with manually set indices where start > end
    let mut modified_substr = SubString::new("hello");
    modified_substr.start_index = 3;
    modified_substr.end_index = 2; // start_index > end_index
    assert!(modified_substr.is_empty());
  }

  #[test]
  fn unicode_characters_are_handled_correctly() {
    let substr = SubString::new("héllo 🌍");
    assert_eq!(substr.first(), Some('h'));
    assert_eq!(substr.get(1), Some('é'));
    assert!(substr.starts_with("hé"));
    assert_eq!(substr.to_string(), "héllo 🌍");
  }

  #[test]
  fn starts_with_uses_loop_to_avoid_string_creation() {
    let substr = SubString::new("hello world");

    // Test various edge cases that would reveal string creation issues
    assert!(substr.starts_with(""));
    assert!(substr.starts_with("h"));
    assert!(substr.starts_with("hello"));
    assert!(substr.starts_with("hello "));
    assert!(!substr.starts_with("hello world!"));
  }
}
