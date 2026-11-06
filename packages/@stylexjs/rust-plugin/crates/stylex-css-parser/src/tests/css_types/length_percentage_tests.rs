// Tests extracted for css_types/length_percentage.rs behaviors and edge cases.
// Source: crates/stylex-css-parser/src/css_types/length_percentage.rs

use super::*;

#[test]
fn test_length_percentage_creation() {
  let length = LengthPercentage::Length(Length::new(16.0, "px".to_string()));
  let percentage = LengthPercentage::Percentage(Percentage::new(50.0));

  assert!(length.is_length());
  assert!(!length.is_percentage());

  assert!(!percentage.is_length());
  assert!(percentage.is_percentage());
}

#[test]
fn test_length_percentage_display() {
  let length = LengthPercentage::Length(Length::new(16.0, "px".to_string()));
  assert_eq!(length.to_string(), "16px");

  let percentage = LengthPercentage::Percentage(Percentage::new(50.0));
  assert_eq!(percentage.to_string(), "50%");

  let zero_length = LengthPercentage::Length(Length::new(0.0, String::new()));
  assert_eq!(zero_length.to_string(), "0");
}

#[test]
fn test_length_percentage_accessors() {
  let length_val = Length::new(20.0, "em".to_string());
  let percentage_val = Percentage::new(75.0);

  let length = LengthPercentage::Length(length_val.clone());
  let percentage = LengthPercentage::Percentage(percentage_val.clone());

  // Test as_length
  assert_eq!(length.as_length(), Some(&length_val));
  assert_eq!(percentage.as_length(), None);

  // Test as_percentage
  assert_eq!(length.as_percentage(), None);
  assert_eq!(percentage.as_percentage(), Some(&percentage_val));
}

#[test]
fn test_length_percentage_parser_creation() {
  // Test both creation methods
  let _parser1 = LengthPercentage::parser();
  let _parser2 = length_percentage_parser();
}

#[test]
fn test_length_percentage_equality() {
  let length1 = LengthPercentage::Length(Length::new(16.0, "px".to_string()));
  let length2 = LengthPercentage::Length(Length::new(16.0, "px".to_string()));
  let length3 = LengthPercentage::Length(Length::new(20.0, "px".to_string()));

  let percentage1 = LengthPercentage::Percentage(Percentage::new(50.0));
  let percentage2 = LengthPercentage::Percentage(Percentage::new(50.0));

  assert_eq!(length1, length2);
  assert_ne!(length1, length3);
  assert_ne!(length1, percentage1);
  assert_eq!(percentage1, percentage2);
}

#[test]
fn test_length_percentage_edge_cases() {
  // Zero length
  let zero_length = LengthPercentage::Length(Length::new(0.0, String::new()));
  assert!(zero_length.is_length());
  assert_eq!(zero_length.to_string(), "0");

  // Zero percentage
  let zero_percentage = LengthPercentage::Percentage(Percentage::new(0.0));
  assert!(zero_percentage.is_percentage());
  assert_eq!(zero_percentage.to_string(), "0%");

  // Different length units
  let px_length = LengthPercentage::Length(Length::new(16.0, "px".to_string()));
  let em_length = LengthPercentage::Length(Length::new(1.0, "em".to_string()));
  assert_ne!(px_length, em_length);
}
