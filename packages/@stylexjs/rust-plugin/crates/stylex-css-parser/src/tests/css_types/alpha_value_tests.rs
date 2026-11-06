// Tests extracted for css_types/alpha_value.rs behaviors and edge cases.
// Source: crates/stylex-css-parser/src/css_types/alpha_value.rs

use super::*;

#[test]
fn test_alpha_value_creation() {
  let alpha1 = AlphaValue::new(0.5);
  let alpha2 = AlphaValue::new(1.0);
  let alpha3 = AlphaValue::new(0.0);

  assert_eq!(alpha1.value, 0.5);
  assert_eq!(alpha2.value, 1.0);
  assert_eq!(alpha3.value, 0.0);
}

#[test]
fn test_alpha_display() {
  let alpha = AlphaValue::new(0.75);
  assert_eq!(alpha.to_string(), "0.75");
}

#[test]
fn test_parser_creation() {
  // Test that the parser can be created
  let _parser = AlphaValue::parser();
  let _alpha_as_num = alpha_as_number();
}

#[test]
fn test_alpha_value_equality() {
  let alpha1 = AlphaValue::new(0.5);
  let alpha2 = AlphaValue::new(0.5);
  let alpha3 = AlphaValue::new(0.75);

  assert_eq!(alpha1, alpha2);
  assert_ne!(alpha1, alpha3);
}
