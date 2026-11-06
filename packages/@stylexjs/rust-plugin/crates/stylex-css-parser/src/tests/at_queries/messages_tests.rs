// Tests extracted for at_queries/messages.rs behaviors and edge cases.
// Source: crates/stylex-css-parser/src/at_queries/messages.rs

use super::*;

#[test]
fn test_error_message_content() {
  assert!(MediaQueryErrors::SYNTAX_ERROR.contains("syntax"));
  assert!(MediaQueryErrors::UNBALANCED_PARENS.contains("parentheses"));
}

#[test]
fn test_error_message_content_validation() {
  assert_eq!(
    MediaQueryErrors::SYNTAX_ERROR,
    "Invalid media query syntax."
  );
  assert_eq!(
    MediaQueryErrors::UNBALANCED_PARENS,
    "Unbalanced parentheses in media query."
  );
}
