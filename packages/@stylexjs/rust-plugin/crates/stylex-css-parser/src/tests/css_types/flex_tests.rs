// Tests extracted for css_types/flex.rs behaviors and edge cases.
// Source: crates/stylex-css-parser/src/css_types/flex.rs

use super::*;

#[test]
fn test_flex_creation() {
  let flex = Flex::new(1.0);
  assert_eq!(flex.fraction, 1.0);

  let flex_half = Flex::new(0.5);
  assert_eq!(flex_half.fraction, 0.5);

  let flex_zero = Flex::new(0.0);
  assert_eq!(flex_zero.fraction, 0.0);
}

#[test]
fn test_flex_display() {
  assert_eq!(Flex::new(1.0).to_string(), "1fr");
  assert_eq!(Flex::new(2.5).to_string(), "2.5fr");
  assert_eq!(Flex::new(0.0).to_string(), "0fr");
  assert_eq!(Flex::new(10.0).to_string(), "10fr");
}

#[test]
fn test_flex_equality() {
  let flex1 = Flex::new(1.0);
  let flex2 = Flex::new(1.0);
  let flex3 = Flex::new(2.0);

  assert_eq!(flex1, flex2);
  assert_ne!(flex1, flex3);
}

#[test]
fn test_is_valid_fraction() {
  assert!(Flex::is_valid_fraction(0.0));
  assert!(Flex::is_valid_fraction(1.0));
  assert!(Flex::is_valid_fraction(2.5));
  assert!(Flex::is_valid_fraction(100.0));

  // Negative values should be invalid
  assert!(!Flex::is_valid_fraction(-1.0));
  assert!(!Flex::is_valid_fraction(-0.5));
}

#[test]
fn test_flex_parser_creation() {
  // Basic test that parser can be created
  let _parser = Flex::parser();
}

#[test]
fn test_flex_common_values() {
  // Test common flex fraction values
  let one_fr = Flex::new(1.0);
  assert_eq!(one_fr.to_string(), "1fr");

  let two_fr = Flex::new(2.0);
  assert_eq!(two_fr.to_string(), "2fr");

  let half_fr = Flex::new(0.5);
  assert_eq!(half_fr.to_string(), "0.5fr");
}

#[test]
fn test_flex_precision() {
  // Test fractional values with precision
  let precise_flex = Flex::new(1.25);
  assert_eq!(precise_flex.to_string(), "1.25fr");

  let small_flex = Flex::new(0.1);
  assert_eq!(small_flex.to_string(), "0.1fr");
}

#[test]
fn test_flex_grid_layout_values() {
  // Test typical grid layout flex values
  let equal_columns = Flex::new(1.0); // 1fr for equal columns
  assert_eq!(equal_columns.to_string(), "1fr");

  let larger_column = Flex::new(2.0); // 2fr for larger column
  assert_eq!(larger_column.to_string(), "2fr");

  let smaller_column = Flex::new(0.5); // 0.5fr for smaller column
  assert_eq!(smaller_column.to_string(), "0.5fr");
}

#[test]
fn test_flex_zero_value() {
  // Test zero flex value (should be valid)
  let zero_flex = Flex::new(0.0);
  assert_eq!(zero_flex.fraction, 0.0);
  assert_eq!(zero_flex.to_string(), "0fr");
  assert!(Flex::is_valid_fraction(0.0));
}

#[test]
fn test_flex_large_values() {
  // Test large flex values
  let large_flex = Flex::new(100.0);
  assert_eq!(large_flex.to_string(), "100fr");

  let very_large_flex = Flex::new(1000.0);
  assert_eq!(very_large_flex.to_string(), "1000fr");
}

#[test]
fn test_flex_decimal_precision() {
  // Test various decimal precisions
  let three_decimals = Flex::new(1.125);
  assert_eq!(three_decimals.to_string(), "1.125fr");

  let many_decimals = Flex::new(1.234_567_9);
  // Note: Display might round or truncate, but value should be preserved
  assert_eq!(many_decimals.fraction, 1.234_567_9);
}
