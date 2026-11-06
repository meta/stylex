//! Focused tests for high-impact shared regex patterns.

use crate::regex::{NPM_NAME_REGEX, SANITIZE_CLASS_NAME_REGEX};

/// Ensure invalid class-name characters are detected while valid names pass.
#[test]
fn sanitize_class_name_regex_matches_invalid_characters() {
  assert!(SANITIZE_CLASS_NAME_REGEX.is_match("a b").unwrap());
  assert!(!SANITIZE_CLASS_NAME_REGEX.is_match("valid_name-1").unwrap());
}

/// Ensure package names accepted by npm naming conventions are matched.
#[test]
fn npm_name_regex_accepts_valid_scoped_names() {
  assert!(NPM_NAME_REGEX.is_match("@scope/pkg-name").unwrap());
  assert!(NPM_NAME_REGEX.is_match("plain-package").unwrap());
  assert!(!NPM_NAME_REGEX.is_match("Invalid Package").unwrap());
}
