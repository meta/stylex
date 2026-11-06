//! Coverage and behavioral tests for shared constants used by the transform
//! pipeline.

use crate::constants::common::{
  COLOR_FUNCTION_LISTED_NORMALIZED_PROPERTY_VALUES, COLOR_RELATIVE_VALUE_FUNCTIONS, COMPILED_KEY,
  CONSTS_FILE_EXTENSION, CSS_CONTENT_FUNCTIONS, CSS_CONTENT_KEYWORDS, DEFAULT_INJECT_PATH,
  LOGICAL_FLOAT_END_VAR, LOGICAL_FLOAT_START_VAR, MUTATING_ARRAY_METHODS, MUTATING_OBJECT_METHODS,
  ROOT_FONT_SIZE, RUNTIME_JSX_CALL_NAMES, SPLIT_TOKEN, VALID_ARRAY_METHODS, VALID_CALLEES,
  VALID_MATH_METHODS, VALID_NUMBER_METHODS, VALID_OBJECT_METHODS, VALID_POSITION_TRY_PROPERTIES,
  VALID_STRING_METHODS, VALID_VIEW_TRANSITION_CLASS_PROPERTIES, VAR_GROUP_HASH_KEY,
};

/// Scalar constants should remain stable because they are externally consumed
/// identifiers.
#[test]
fn scalar_constants_match_expected_contracts() {
  assert_eq!(DEFAULT_INJECT_PATH, "@stylexjs/stylex/lib/stylex-inject");
  assert_eq!(COMPILED_KEY, "$$css");
  assert_eq!(SPLIT_TOKEN, "__$$__");
  assert_eq!(ROOT_FONT_SIZE, 16);
  assert_eq!(VAR_GROUP_HASH_KEY, "__varGroupHash__");
  assert_eq!(CONSTS_FILE_EXTENSION, ".const");
  assert_eq!(LOGICAL_FLOAT_START_VAR, "--stylex-logical-start");
  assert_eq!(LOGICAL_FLOAT_END_VAR, "--stylex-logical-end");
}

/// Method allow/deny sets should expose expected representative entries.
#[test]
fn method_sets_include_expected_members() {
  assert!(VALID_CALLEES.contains("String"));
  assert!(VALID_CALLEES.contains("Array"));

  assert!(MUTATING_ARRAY_METHODS.contains("push"));
  assert!(MUTATING_ARRAY_METHODS.contains("splice"));

  assert!(MUTATING_OBJECT_METHODS.contains("assign"));
  assert!(MUTATING_OBJECT_METHODS.contains("defineProperty"));
}

/// The per-global allowlists, asked for a static each global carries and for
/// one it does not.
#[test]
fn static_allowlists_include_expected_members() {
  assert!(VALID_STRING_METHODS.contains("fromCharCode"));
  assert!(VALID_STRING_METHODS.contains("raw"));
  assert_eq!(VALID_STRING_METHODS.len(), 3);

  assert!(VALID_NUMBER_METHODS.contains("isInteger"));
  assert!(VALID_NUMBER_METHODS.contains("parseInt"));
  assert_eq!(VALID_NUMBER_METHODS.len(), 6);

  assert!(VALID_MATH_METHODS.contains("max"));
  assert!(VALID_MATH_METHODS.contains("f16round"));
  assert_eq!(VALID_MATH_METHODS.len(), 35);

  assert!(VALID_OBJECT_METHODS.contains("keys"));
  assert!(VALID_OBJECT_METHODS.contains("groupBy"));
  assert_eq!(VALID_OBJECT_METHODS.len(), 12);

  assert!(VALID_ARRAY_METHODS.contains("from"));
  assert!(VALID_ARRAY_METHODS.contains("isArray"));
  assert_eq!(VALID_ARRAY_METHODS.len(), 3);
}

/// What the allowlists must not hold: a static that answers anew on every
/// build, one that changes what it was handed, and one that answers with an
/// object from the prototype chain.
#[test]
fn static_allowlists_exclude_the_statics_that_are_refused() {
  assert!(!VALID_MATH_METHODS.contains("random"));

  for method in MUTATING_OBJECT_METHODS.iter() {
    assert!(
      !VALID_OBJECT_METHODS.contains(method),
      "`Object.{}` mutates and must stay outside the allowlist",
      method
    );
  }

  for method in [
    "create",
    "getPrototypeOf",
    "setPrototypeOf",
    "getOwnPropertyDescriptor",
    "getOwnPropertyDescriptors",
  ] {
    assert!(
      !VALID_OBJECT_METHODS.contains(method),
      "`Object.{}` is reflective and must stay outside the allowlist",
      method
    );
  }
}

/// Lazy lists used by normalizers and runtime checks should be initialized and
/// validated.
#[test]
fn lazy_constant_lists_have_expected_shape() {
  assert_eq!(COLOR_FUNCTION_LISTED_NORMALIZED_PROPERTY_VALUES.len(), 9);
  assert!(COLOR_FUNCTION_LISTED_NORMALIZED_PROPERTY_VALUES.contains(&"oklch"));
  assert!(COLOR_FUNCTION_LISTED_NORMALIZED_PROPERTY_VALUES.contains(&"radial-gradient"));

  assert_eq!(COLOR_RELATIVE_VALUE_FUNCTIONS.len(), 10);
  assert!(COLOR_RELATIVE_VALUE_FUNCTIONS.contains(&"rgb"));
  assert!(COLOR_RELATIVE_VALUE_FUNCTIONS.contains(&"color"));

  assert_eq!(CSS_CONTENT_FUNCTIONS.len(), 7);
  assert!(CSS_CONTENT_FUNCTIONS.contains(&"url("));
  assert!(CSS_CONTENT_FUNCTIONS.contains(&"var(--"));

  assert_eq!(CSS_CONTENT_KEYWORDS.len(), 11);
  assert!(CSS_CONTENT_KEYWORDS.contains(&"normal"));
  assert!(CSS_CONTENT_KEYWORDS.contains(&"revert-layer"));
}

/// Position/transition property allow-lists should keep their canonical
/// entries.
#[test]
fn property_allow_lists_stay_consistent() {
  assert_eq!(VALID_POSITION_TRY_PROPERTIES.len(), 40);
  assert!(VALID_POSITION_TRY_PROPERTIES.contains(&"top"));
  assert!(VALID_POSITION_TRY_PROPERTIES.contains(&"placeSelf"));

  assert_eq!(VALID_VIEW_TRANSITION_CLASS_PROPERTIES.len(), 4);
  assert!(VALID_VIEW_TRANSITION_CLASS_PROPERTIES.contains(&"group"));
  assert!(VALID_VIEW_TRANSITION_CLASS_PROPERTIES.contains(&"new"));
}

/// Runtime JSX helper names should include both classic and modern factory
/// calls.
#[test]
fn runtime_jsx_call_names_include_expected_variants() {
  assert!(RUNTIME_JSX_CALL_NAMES.contains(&"jsx"));
  assert!(RUNTIME_JSX_CALL_NAMES.contains(&"createElement"));
  assert!(RUNTIME_JSX_CALL_NAMES.contains(&"createVNode"));
}
