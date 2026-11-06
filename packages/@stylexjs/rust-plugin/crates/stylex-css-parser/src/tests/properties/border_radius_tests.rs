// Tests extracted for properties/border_radius.rs behaviors and edge cases.
// Source: crates/stylex-css-parser/src/properties/border_radius.rs

use super::*;
use crate::css_types::{Length, Percentage};

#[test]
fn test_border_radius_individual_creation() {
  let length = LengthPercentage::Length(Length::new(10.0, "px".to_string()));
  let radius = BorderRadiusIndividual::new(length.clone(), None);

  assert_eq!(radius.horizontal, length);
  assert_eq!(radius.vertical, length);
}

#[test]
fn test_border_radius_individual_different_values() {
  let horizontal = LengthPercentage::Length(Length::new(10.0, "px".to_string()));
  let vertical = LengthPercentage::Percentage(Percentage::new(50.0));
  let radius = BorderRadiusIndividual::new(horizontal.clone(), Some(vertical.clone()));

  assert_eq!(radius.horizontal, horizontal);
  assert_eq!(radius.vertical, vertical);
}

#[test]
fn test_border_radius_individual_display() {
  let length = LengthPercentage::Length(Length::new(5.0, "px".to_string()));
  let radius = BorderRadiusIndividual::new(length, None);
  assert_eq!(radius.to_string(), "5px");

  let horizontal = LengthPercentage::Length(Length::new(10.0, "px".to_string()));
  let vertical = LengthPercentage::Percentage(Percentage::new(20.0));
  let radius2 = BorderRadiusIndividual::new(horizontal, Some(vertical));
  assert_eq!(radius2.to_string(), "10px 20%");
}

#[test]
fn test_border_radius_shorthand_creation() {
  let value = LengthPercentage::Length(Length::new(5.0, "px".to_string()));
  let shorthand =
    BorderRadiusShorthand::new(value.clone(), None, None, None, None, None, None, None);

  // All corners should be the same
  assert_eq!(shorthand.horizontal_top_left, value);
  assert_eq!(shorthand.horizontal_top_right, value);
  assert_eq!(shorthand.horizontal_bottom_right, value);
  assert_eq!(shorthand.horizontal_bottom_left, value);
  assert_eq!(shorthand.vertical_top_left, value);
  assert_eq!(shorthand.vertical_top_right, value);
  assert_eq!(shorthand.vertical_bottom_right, value);
  assert_eq!(shorthand.vertical_bottom_left, value);
}

#[test]
fn test_border_radius_shorthand_display_single_value() {
  let value = LengthPercentage::Length(Length::new(5.0, "px".to_string()));
  let shorthand = BorderRadiusShorthand::new(value, None, None, None, None, None, None, None);

  assert_eq!(shorthand.to_string(), "5px");
}

#[test]
fn test_border_radius_shorthand_css_expansion() {
  let top_left = LengthPercentage::Length(Length::new(1.0, "px".to_string()));
  let top_right = LengthPercentage::Length(Length::new(2.0, "px".to_string()));

  let shorthand = BorderRadiusShorthand::new(
    top_left.clone(),
    Some(top_right.clone()),
    None, // Should default to top_left
    None, // Should default to top_right
    None,
    None,
    None,
    None,
  );

  assert_eq!(shorthand.horizontal_top_left, top_left);
  assert_eq!(shorthand.horizontal_top_right, top_right);
  assert_eq!(shorthand.horizontal_bottom_right, top_left); // Defaults to top_left
  assert_eq!(shorthand.horizontal_bottom_left, top_right); // Defaults to top_right
}

#[test]
fn test_border_radius_individual_parser_creation() {
  // Basic test that parser can be created
  let _parser = BorderRadiusIndividual::parser();
}

#[test]
fn test_border_radius_shorthand_parser_creation() {
  // Basic test that parser can be created
  let _parser = BorderRadiusShorthand::parser();
}

#[test]
fn test_border_radius_equality() {
  let value1 = LengthPercentage::Length(Length::new(5.0, "px".to_string()));
  let value2 = LengthPercentage::Length(Length::new(5.0, "px".to_string()));
  let value3 = LengthPercentage::Length(Length::new(10.0, "px".to_string()));

  let radius1 = BorderRadiusIndividual::new(value1, None);
  let radius2 = BorderRadiusIndividual::new(value2, None);
  let radius3 = BorderRadiusIndividual::new(value3, None);

  assert_eq!(radius1, radius2);
  assert_ne!(radius1, radius3);
}

#[test]
fn test_border_radius_common_css_values() {
  // Test common border-radius values
  let small = LengthPercentage::Length(Length::new(3.0, "px".to_string()));
  let medium = LengthPercentage::Length(Length::new(6.0, "px".to_string()));
  let large = LengthPercentage::Length(Length::new(12.0, "px".to_string()));
  let circle = LengthPercentage::Percentage(Percentage::new(50.0));

  let small_radius = BorderRadiusIndividual::new(small, None);
  assert_eq!(small_radius.to_string(), "3px");

  let medium_radius = BorderRadiusIndividual::new(medium, None);
  assert_eq!(medium_radius.to_string(), "6px");

  let large_radius = BorderRadiusIndividual::new(large, None);
  assert_eq!(large_radius.to_string(), "12px");

  let circle_radius = BorderRadiusIndividual::new(circle, None);
  assert_eq!(circle_radius.to_string(), "50%");
}

#[test]
fn test_border_radius_elliptical() {
  // Test elliptical border radius (different horizontal and vertical)
  let horizontal = LengthPercentage::Length(Length::new(20.0, "px".to_string()));
  let vertical = LengthPercentage::Length(Length::new(10.0, "px".to_string()));

  let elliptical = BorderRadiusIndividual::new(horizontal, Some(vertical));
  assert_eq!(elliptical.to_string(), "20px 10px");
}

#[test]
fn test_border_radius_mixed_units() {
  // Test mixing different units
  let pixels = LengthPercentage::Length(Length::new(5.0, "px".to_string()));
  let percentage = LengthPercentage::Percentage(Percentage::new(25.0));

  let mixed = BorderRadiusIndividual::new(pixels, Some(percentage));
  assert_eq!(mixed.to_string(), "5px 25%");
}

#[test]
fn test_border_radius_shorthand_slash_separated() {
  // Test asymmetric border radius: 10px 20px / 5px 15px
  let h_tl = LengthPercentage::Length(Length::new(10.0, "px".to_string()));
  let h_tr = LengthPercentage::Length(Length::new(20.0, "px".to_string()));
  let v_tl = LengthPercentage::Length(Length::new(5.0, "px".to_string()));
  let v_tr = LengthPercentage::Length(Length::new(15.0, "px".to_string()));

  let radius = BorderRadiusShorthand::new(
    h_tl,
    Some(h_tr),
    None,
    None,
    Some(v_tl),
    Some(v_tr),
    None,
    None,
  );

  // Should output the slash-separated format when horizontal and vertical differ
  let result = radius.to_string();
  assert!(result.contains("/"));
  assert!(result.contains("10px"));
  assert!(result.contains("20px"));
  assert!(result.contains("5px"));
  assert!(result.contains("15px"));
}

#[test]
fn test_border_radius_shorthand_no_slash_when_same() {
  // Test when horizontal and vertical radii are the same, no slash should appear
  let value = LengthPercentage::Length(Length::new(10.0, "px".to_string()));

  let radius = BorderRadiusShorthand::new(
    value.clone(),
    None,
    None,
    None,
    Some(value),
    None,
    None,
    None,
  );

  let result = radius.to_string();
  assert!(!result.contains("/"));
  assert_eq!(result, "10px");
}

#[test]
fn test_border_radius_parser_creation() {
  // Test that both parsers can be created without issues
  let _individual = BorderRadiusIndividual::parser();
  let _shorthand = BorderRadiusShorthand::parser();
}
