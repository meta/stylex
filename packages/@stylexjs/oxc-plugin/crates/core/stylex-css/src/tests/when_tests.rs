// Tests for pseudo-selector observer helpers and marker generation.
// Source: crates/stylex-css/src/utils/when.rs

use super::*;

#[test]
fn test_validate_pseudo_selector_invalid_attribute_nested_bracket() {
  let result = validate_pseudo_selector("[data-[nested]=\"open\"]");
  assert!(result.is_err());
  assert!(result.unwrap_err().contains("invalid format"));
}

/// `[a][b]`: covers the `bracket_count <= 1` branch of the nesting guard.
#[test]
fn test_validate_pseudo_selector_sequential_attribute_selectors() {
  let result = validate_pseudo_selector("[a][b]");
  assert!(result.is_ok());
}

// --- ancestor additional tests ---

// --- descendant additional tests ---

// --- sibling_before additional tests ---

// --- sibling_after additional tests ---

// --- any_sibling additional tests ---

// --- from_proxy / from_stylex_style ---

// --- get_default_marker_class_name indirect tests ---

// --- Validate attribute selectors with functions ---

#[test]
fn ancestor_with_attribute_value_selector() {
  let result = ancestor("[data-theme='dark']", None).unwrap();
  assert_eq!(result, ":where(.x-default-marker[data-theme='dark'] *)");
}

// --- Validate invalid attribute selectors with functions ---

// --- is_valid_attribute_selector edge cases ---

#[test]
fn is_valid_attribute_selector_empty() {
  assert!(!is_valid_attribute_selector(""));
}

#[test]
fn is_valid_attribute_selector_close_bracket_mid_string() {
  assert!(!is_valid_attribute_selector("[a]b]"));
}

// --- Additional pseudo selectors ---

// --- custom marker resolution ---

/// Stands in for the evaluated values the compiler passes as the second argument.
#[derive(Default)]
struct TestMarker {
  str_value: Option<String>,
  proxy_string: Option<String>,
  css_key: Option<String>,
}

impl TestMarker {
  fn str_value(value: &str) -> Self {
    Self {
      str_value: Some(value.to_string()),
      ..Default::default()
    }
  }
}

impl WhenMarkerValue for TestMarker {
  fn as_str_value(&self) -> Option<&str> {
    self.str_value.as_deref()
  }

  fn is_proxy(&self) -> bool {
    self.proxy_string.is_some()
  }

  fn as_proxy_string(&self) -> Option<String> {
    self.proxy_string.clone()
  }

  fn first_css_key(&self) -> Option<&str> {
    self.css_key.as_deref()
  }

  fn class_name_prefix(&self) -> Option<&str> {
    None
  }
}

#[test]
fn descendant_with_string_marker() {
  let marker = TestMarker::str_value("myMarker");
  let result = descendant(":focus", Some(&marker)).unwrap();
  assert_eq!(result, ":where(:has(.myMarker:focus))");
}

/// Markers carry no `classNamePrefix`, so the fallback is unprefixed.
#[test]
fn ancestor_with_unresolvable_marker_falls_back_unprefixed() {
  let marker = TestMarker::default();
  let result = ancestor(":hover", Some(&marker)).unwrap();
  assert_eq!(result, ":where(.default-marker:hover *)");
}
