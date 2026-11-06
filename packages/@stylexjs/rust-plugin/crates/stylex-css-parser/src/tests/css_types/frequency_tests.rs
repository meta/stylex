// Tests extracted for css_types/frequency.rs behaviors and edge cases.
// Source: crates/stylex-css-parser/src/css_types/frequency.rs

use super::*;

#[test]
fn test_frequency_creation() {
  let freq = Frequency::new(440.0, "Hz".to_string());
  assert_eq!(freq.value, 440.0);
  assert_eq!(freq.unit, "Hz");
}

#[test]
fn test_frequency_display() {
  let hertz = Frequency::new(440.0, "Hz".to_string());
  assert_eq!(hertz.to_string(), "0.44KHz");

  let kilohertz = Frequency::new(2.4, "KHz".to_string());
  assert_eq!(kilohertz.to_string(), "2.4KHz");

  let full_kilohertz = Frequency::new(1000.0, "Hz".to_string());
  assert_eq!(full_kilohertz.to_string(), "1KHz");

  let two_kilohertz = Frequency::new(2000.0, "Hz".to_string());
  assert_eq!(two_kilohertz.to_string(), "2KHz");

  let partial = Frequency::new(1500.0, "Hz".to_string());
  assert_eq!(partial.to_string(), "1.5KHz");
}

#[test]
fn test_valid_frequency_units() {
  assert!(Frequency::is_valid_unit("Hz"));
  assert!(Frequency::is_valid_unit("KHz"));

  // Invalid units
  assert!(!Frequency::is_valid_unit("px"));
  assert!(!Frequency::is_valid_unit("s"));
  assert!(!Frequency::is_valid_unit("deg"));
}

#[test]
fn test_frequency_units_constant() {
  let units = Frequency::units();
  assert_eq!(units.len(), 2);
  assert!(units.contains(&"Hz"));
  assert!(units.contains(&"KHz"));
}

#[test]
fn test_frequency_parser_creation() {
  // Basic test that parser can be created
  let _parser = Frequency::parser();
}
