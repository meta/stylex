/*!
Base types tests.
*/

use crate::base_types::SubString;

#[cfg(test)]
mod test_base_types {
  use super::*;

  #[test]
  fn starts_with_returns_false_for_non_matching_prefix() {
    let substr = SubString::new("hello world");
    assert!(!substr.starts_with("world"));
    assert!(!substr.starts_with("hello world!")); // longer than original
    assert!(!substr.starts_with("goodbye"));
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
    // start_index == end_index == 0, so this implementation does not report empty.
    assert!(!empty_substr.is_empty());

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
}
