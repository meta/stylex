// Tests extracted for css_types/time.rs behaviors and edge cases.
// Source: crates/stylex-css-parser/src/css_types/time.rs

use super::*;

#[test]
fn test_time_creation() {
  let time = Time::new(1.5, "s".to_string());
  assert_eq!(time.value, 1.5);
  assert_eq!(time.unit, "s");
}

#[test]
fn test_time_display() {
  let seconds = Time::new(1.5, "s".to_string());
  assert_eq!(seconds.to_string(), "1.5s");

  let milliseconds = Time::new(500.0, "ms".to_string());
  assert_eq!(milliseconds.to_string(), "0.5s");

  let full_second = Time::new(1000.0, "ms".to_string());
  assert_eq!(full_second.to_string(), "1s");

  let two_seconds = Time::new(2000.0, "ms".to_string());
  assert_eq!(two_seconds.to_string(), "2s");

  let partial = Time::new(1500.0, "ms".to_string());
  assert_eq!(partial.to_string(), "1.5s");
}

#[test]
fn test_valid_time_units() {
  assert!(Time::is_valid_unit("s"));
  assert!(Time::is_valid_unit("ms"));

  // Invalid units
  assert!(!Time::is_valid_unit("px"));
  assert!(!Time::is_valid_unit("deg"));
  assert!(!Time::is_valid_unit("Hz"));
}

#[test]
fn test_time_units_constant() {
  let units = Time::units();
  assert_eq!(units.len(), 2);
  assert!(units.contains(&"s"));
  assert!(units.contains(&"ms"));
}

#[test]
fn test_time_parser_creation() {
  // Basic test that parser can be created
  let _parser = Time::parser();
}
