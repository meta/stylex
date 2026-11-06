// Tests extracted for css_types/resolution.rs behaviors and edge cases.
// Source: crates/stylex-css-parser/src/css_types/resolution.rs

use super::*;

#[test]
fn test_resolution_creation() {
  let res = Resolution::new(96.0, "dpi".to_string());
  assert_eq!(res.value, 96.0);
  assert_eq!(res.unit, "dpi");
}

#[test]
fn test_resolution_display() {
  let dpi = Resolution::new(96.0, "dpi".to_string());
  assert_eq!(dpi.to_string(), "96dpi");

  let dpcm = Resolution::new(38.0, "dpcm".to_string());
  assert_eq!(dpcm.to_string(), "38dpcm");

  let dppx = Resolution::new(2.0, "dppx".to_string());
  assert_eq!(dppx.to_string(), "2dppx");
}

#[test]
fn test_valid_resolution_units() {
  assert!(Resolution::is_valid_unit("dpi"));
  assert!(Resolution::is_valid_unit("dpcm"));
  assert!(Resolution::is_valid_unit("dppx"));

  // Invalid units
  assert!(!Resolution::is_valid_unit("px"));
  assert!(!Resolution::is_valid_unit("s"));
  assert!(!Resolution::is_valid_unit("Hz"));
  assert!(!Resolution::is_valid_unit("deg"));
}

#[test]
fn test_resolution_units_constant() {
  let units = Resolution::units();
  assert_eq!(units.len(), 3);
  assert!(units.contains(&"dpi"));
  assert!(units.contains(&"dpcm"));
  assert!(units.contains(&"dppx"));
}

#[test]
fn test_resolution_parser_creation() {
  // Basic test that parser can be created
  let _parser = Resolution::parser();
}
