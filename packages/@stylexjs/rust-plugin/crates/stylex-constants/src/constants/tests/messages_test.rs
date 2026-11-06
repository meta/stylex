//! Tests for user-facing message generation functions (argument length, static
//! values, etc).

use crate::constants::messages::*;

#[test]
fn test_illegal_argument_length_singular() {
  assert_eq!(
    illegal_argument_length("create", 1),
    "create() should have 1 argument."
  );
}

#[test]
fn test_illegal_argument_length_plural() {
  assert_eq!(
    illegal_argument_length("create", 2),
    "create() should have 2 arguments."
  );
}

#[test]
fn test_non_static_value() {
  assert_eq!(
    non_static_value("create"),
    "Only static values are allowed inside of a create() call."
  );
}

#[test]
fn test_non_static_value_with_vowel_api_name() {
  assert_eq!(
    non_static_value("unstable_defineVarsNested"),
    "Only static values are allowed inside of an unstable_defineVarsNested() call."
  );
}

#[test]
fn test_non_static_value_with_empty_api_name_falls_back_to_a() {
  // Defensive: covers the `None` branch of `indefinite_article` so an empty
  // API name still produces a grammatically valid (if useless) sentence rather
  // than panicking.
  assert_eq!(
    non_static_value(""),
    "Only static values are allowed inside of a () call."
  );
}

#[test]
fn test_non_style_object() {
  assert_eq!(
    non_style_object("create"),
    "create() can only accept an object."
  );
}

#[test]
fn test_non_export_named_declaration() {
  assert_eq!(
    non_export_named_declaration("create"),
    "The return value of create() must be bound to a named export."
  );
}

#[test]
fn test_unbound_call_value() {
  assert_eq!(
    unbound_call_value("create"),
    "create() calls must be bound to a bare variable."
  );
}

#[test]
fn test_type_asserted_call_value() {
  assert_eq!(
    type_asserted_call_value("create"),
    "create() cannot be written inside a type assertion. Bind the call to a variable and assert the variable."
  );
}

#[test]
fn test_cannot_generate_hash() {
  let result = cannot_generate_hash("create");
  assert!(result.starts_with("Unable to generate hash for create()"));
}

#[test]
fn test_expected_call_expression() {
  let result = expected_call_expression("defineVars");
  assert!(result.starts_with("defineVars(): Expected a call expression"));
}

#[test]
fn test_invalid_define_vars_function_value() {
  let result = invalid_define_vars_function_value();
  assert!(result.contains("zero-argument"));
}

#[test]
fn test_cyclic_define_vars_reference() {
  let result = cyclic_define_vars_reference("a -> b -> a");
  assert_eq!(
    result,
    "Cyclic same-group references in defineVars() are not allowed: a -> b -> a."
  );
}

#[test]
fn test_unknown_define_vars_reference() {
  let result = unknown_define_vars_reference("textMuted", "missing");
  assert_eq!(
    result,
    "Unknown same-group reference \"missing\" found while resolving \"textMuted\" in defineVars()."
  );
}

/// The refusal a `defineVars()` variable gets when its object carries no
/// `default` key.
///
/// Byte-identical to `'Default value is not defined for ' + key + ' variable.'`,
/// which upstream builds at `shared/stylex-vars-utils.js:45` and again at
/// `visitors/stylex-define-vars.js:313` (0.19.0). Two details are load-bearing
/// and both used to be wrong here. The name is *unquoted*, which is the whole of
/// what a build error read differently between the two compilers for this input;
/// and the key is the top-level variable rather than the nested key the
/// recursion is standing on, which is why a nested case answers the root name.
#[test]
fn test_missing_default_value_names_the_variable_unquoted() {
  assert_eq!(
    missing_default_value("--my-var"),
    "Default value is not defined for --my-var variable."
  );
  assert_eq!(
    missing_default_value("primaryColor"),
    "Default value is not defined for primaryColor variable."
  );

  // No quoting, whatever the key looks like.
  assert!(!missing_default_value("k").contains('"'));
  assert!(!missing_default_value("k").contains('\''));
}
