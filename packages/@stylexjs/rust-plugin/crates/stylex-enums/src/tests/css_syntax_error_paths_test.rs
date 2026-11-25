//! Error-path tests for CSSSyntax conversion.

use crate::css_syntax::CSSSyntax;

/// Unknown syntax strings panic.
#[test]
fn from_string_unknown_variant_panics() {
  let result = std::panic::catch_unwind(|| CSSSyntax::from("<unknown>".to_string()));
  assert!(result.is_err());
}
