//! Focused tests for high-impact shared regex patterns.

use crate::regex::NPM_NAME_REGEX;

/// Ensure package names accepted by npm naming conventions are matched.
#[test]
fn npm_name_regex_accepts_valid_scoped_names() {
  assert!(NPM_NAME_REGEX.is_match("@scope/pkg-name").unwrap());
  assert!(NPM_NAME_REGEX.is_match("plain-package").unwrap());
  assert!(!NPM_NAME_REGEX.is_match("Invalid Package").unwrap());
}
