// Tests extracted for css_types/angle.rs behaviors and edge cases.
// Source: crates/stylex-css-parser/src/css_types/angle.rs

use super::*;

#[test]
fn test_angle_creation() {
  let angle = Angle::new(45.0, "deg".to_string());
  assert_eq!(angle.value, 45.0);
  assert_eq!(angle.unit, "deg");
}

#[test]
fn test_angle_display() {
  let degrees = Angle::new(45.0, "deg".to_string());
  assert_eq!(degrees.to_string(), "45deg");

  let radians = Angle::new(1.57, "rad".to_string());
  assert_eq!(radians.to_string(), "1.57rad");

  let gradians = Angle::new(100.0, "grad".to_string());
  assert_eq!(gradians.to_string(), "100grad");

  let turns = Angle::new(0.25, "turn".to_string());
  assert_eq!(turns.to_string(), "0.25turn");

  // Zero without unit
  let zero_angle = Angle::new(0.0, "deg".to_string());
  assert_eq!(zero_angle.to_string(), "0deg");
}

#[test]
fn test_valid_angle_units() {
  assert!(Angle::is_valid_unit("deg"));
  assert!(Angle::is_valid_unit("grad"));
  assert!(Angle::is_valid_unit("rad"));
  assert!(Angle::is_valid_unit("turn"));

  // Invalid units
  assert!(!Angle::is_valid_unit("px"));
  assert!(!Angle::is_valid_unit("s"));
  assert!(!Angle::is_valid_unit("Hz"));
  assert!(!Angle::is_valid_unit("invalid"));
}

#[test]
fn test_angle_units_constant() {
  let units = Angle::units();
  assert_eq!(units.len(), 4);
  assert!(units.contains(&"deg"));
  assert!(units.contains(&"grad"));
  assert!(units.contains(&"rad"));
  assert!(units.contains(&"turn"));
}

#[test]
fn test_angle_parser_creation() {
  // Basic test that parser can be created
  let _parser = Angle::parser();
}

#[test]
fn test_angle_equality() {
  let angle1 = Angle::new(45.0, "deg".to_string());
  let angle2 = Angle::new(45.0, "deg".to_string());
  let angle3 = Angle::new(90.0, "deg".to_string());
  let angle4 = Angle::new(45.0, "rad".to_string());

  assert_eq!(angle1, angle2);
  assert_ne!(angle1, angle3);
  assert_ne!(angle1, angle4);
}

#[test]
fn test_angle_units_coverage() {
  // Test all standard CSS angle units are included
  assert!(Angle::is_valid_unit("deg")); // degrees (most common)
  assert!(Angle::is_valid_unit("grad")); // gradians
  assert!(Angle::is_valid_unit("rad")); // radians
  assert!(Angle::is_valid_unit("turn")); // full turns
}
