//! Error-path tests for AST factory helpers.

use crate::ast::factories::create_boolean_prop;

/// `create_boolean_prop` should panic when required boolean value is missing.
#[test]
fn create_boolean_prop_none_panics() {
  let result = std::panic::catch_unwind(|| create_boolean_prop("enabled", None));
  assert!(result.is_err());
}
