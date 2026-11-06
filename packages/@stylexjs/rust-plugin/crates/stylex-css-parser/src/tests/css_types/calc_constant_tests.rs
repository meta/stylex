// Tests extracted for css_types/calc_constant.rs behaviors and edge cases.
// Source: crates/stylex-css-parser/src/css_types/calc_constant.rs

use super::*;

#[test]
fn test_calc_constant_from_str() {
  assert_eq!(CalcConstant::parse("pi"), Some(CalcConstant::Pi));
  assert_eq!(CalcConstant::parse("e"), Some(CalcConstant::E));
  assert_eq!(
    CalcConstant::parse("infinity"),
    Some(CalcConstant::Infinity)
  );
  assert_eq!(
    CalcConstant::parse("-infinity"),
    Some(CalcConstant::NegativeInfinity)
  );
  assert_eq!(CalcConstant::parse("NaN"), Some(CalcConstant::NaN));

  // Invalid constants
  assert_eq!(CalcConstant::parse("invalid"), None);
  assert_eq!(CalcConstant::parse("PI"), None); // Case sensitive
  assert_eq!(CalcConstant::parse(""), None);
}

#[test]
fn test_calc_constant_as_str() {
  assert_eq!(CalcConstant::Pi.as_str(), "pi");
  assert_eq!(CalcConstant::E.as_str(), "e");
  assert_eq!(CalcConstant::Infinity.as_str(), "infinity");
  assert_eq!(CalcConstant::NegativeInfinity.as_str(), "-infinity");
  assert_eq!(CalcConstant::NaN.as_str(), "NaN");
}

#[test]
fn test_calc_constant_display() {
  assert_eq!(CalcConstant::Pi.to_string(), "pi");
  assert_eq!(CalcConstant::E.to_string(), "e");
  assert_eq!(CalcConstant::Infinity.to_string(), "infinity");
  assert_eq!(CalcConstant::NegativeInfinity.to_string(), "-infinity");
  assert_eq!(CalcConstant::NaN.to_string(), "NaN");
}

#[test]
fn test_calc_constant_is_valid() {
  assert!(CalcConstant::is_valid_constant("pi"));
  assert!(CalcConstant::is_valid_constant("e"));
  assert!(CalcConstant::is_valid_constant("infinity"));
  assert!(CalcConstant::is_valid_constant("-infinity"));
  assert!(CalcConstant::is_valid_constant("NaN"));

  // Invalid
  assert!(!CalcConstant::is_valid_constant("invalid"));
  assert!(!CalcConstant::is_valid_constant("PI"));
  assert!(!CalcConstant::is_valid_constant(""));
}

#[test]
fn test_calc_constant_all_constants() {
  let constants = CalcConstant::all_constants();
  assert_eq!(constants.len(), 5);
  assert!(constants.contains(&"pi"));
  assert!(constants.contains(&"e"));
  assert!(constants.contains(&"infinity"));
  assert!(constants.contains(&"-infinity"));
  assert!(constants.contains(&"NaN"));
}

#[test]
fn test_calc_constant_parser_creation() {
  // Basic test that parser can be created
  let _parser = CalcConstant::parser();
}

#[test]
fn test_calc_constant_equality() {
  let pi1 = CalcConstant::Pi;
  let pi2 = CalcConstant::Pi;
  let e = CalcConstant::E;

  assert_eq!(pi1, pi2);
  assert_ne!(pi1, e);
}

#[test]
fn test_calc_constant_round_trip() {
  // Test that parse and as_str are consistent
  for constant_str in CalcConstant::all_constants() {
    let constant = CalcConstant::parse(constant_str).unwrap();
    assert_eq!(constant.as_str(), *constant_str);
  }
}

#[test]
fn test_calc_constant_math_constants() {
  // Test mathematical constants specifically
  assert_eq!(CalcConstant::Pi.as_str(), "pi");
  assert_eq!(CalcConstant::E.as_str(), "e");

  // Test special values
  assert_eq!(CalcConstant::Infinity.as_str(), "infinity");
  assert_eq!(CalcConstant::NegativeInfinity.as_str(), "-infinity");
  assert_eq!(CalcConstant::NaN.as_str(), "NaN");
}
