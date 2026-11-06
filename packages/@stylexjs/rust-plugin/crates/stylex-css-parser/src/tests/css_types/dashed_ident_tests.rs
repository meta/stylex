// Tests extracted for css_types/dashed_ident.rs behaviors and edge cases.
// Source: crates/stylex-css-parser/src/css_types/dashed_ident.rs

use super::*;

#[test]
fn test_dashed_identifier_creation() {
  let ident = DashedIdentifier::new("--my-custom-property".to_string());
  assert_eq!(ident.value, "--my-custom-property");
}

#[test]
fn test_dashed_identifier_display() {
  let ident = DashedIdentifier::new("--primary-color".to_string());
  assert_eq!(ident.to_string(), "--primary-color");
}

#[test]
fn test_valid_dashed_ident() {
  // Valid dashed identifiers
  assert!(DashedIdentifier::is_valid_dashed_ident("--my-variable"));
  assert!(DashedIdentifier::is_valid_dashed_ident("--primary-color"));
  assert!(DashedIdentifier::is_valid_dashed_ident("--font-size"));
  assert!(DashedIdentifier::is_valid_dashed_ident("--bg"));
  assert!(DashedIdentifier::is_valid_dashed_ident("--x"));
  assert!(DashedIdentifier::is_valid_dashed_ident("--123"));
  assert!(DashedIdentifier::is_valid_dashed_ident("--_underscore"));
  assert!(DashedIdentifier::is_valid_dashed_ident("--kebab-case-var"));
  assert!(DashedIdentifier::is_valid_dashed_ident("--camelCaseVar"));

  // Invalid dashed identifiers
  assert!(!DashedIdentifier::is_valid_dashed_ident("-single-dash"));
  assert!(!DashedIdentifier::is_valid_dashed_ident("no-dashes"));
  assert!(!DashedIdentifier::is_valid_dashed_ident("--")); // Only dashes, no name
  assert!(!DashedIdentifier::is_valid_dashed_ident("-")); // Single dash
  assert!(!DashedIdentifier::is_valid_dashed_ident("")); // Empty string
  assert!(!DashedIdentifier::is_valid_dashed_ident("regular-variable"));
}

#[test]
fn test_dashed_identifier_parser_creation() {
  // Basic test that parser can be created
  let _parser = DashedIdentifier::parser();
}

#[test]
fn test_dashed_identifier_equality() {
  let ident1 = DashedIdentifier::new("--my-var".to_string());
  let ident2 = DashedIdentifier::new("--my-var".to_string());
  let ident3 = DashedIdentifier::new("--other-var".to_string());

  assert_eq!(ident1, ident2);
  assert_ne!(ident1, ident3);
}

#[test]
fn test_dashed_identifier_edge_cases() {
  // Minimum valid length (3 characters: --, plus at least 1 more)
  assert!(DashedIdentifier::is_valid_dashed_ident("--a"));
  assert!(DashedIdentifier::is_valid_dashed_ident("--1"));
  assert!(DashedIdentifier::is_valid_dashed_ident("--_"));

  // Just at the boundary
  assert!(!DashedIdentifier::is_valid_dashed_ident("--")); // Too short

  // Multiple dashes should be fine
  assert!(DashedIdentifier::is_valid_dashed_ident("--my--var"));
  assert!(DashedIdentifier::is_valid_dashed_ident("-------"));
}

#[test]
fn test_dashed_identifier_css_custom_properties() {
  // Test common CSS custom property patterns
  let primary_color = DashedIdentifier::new("--primary-color".to_string());
  assert_eq!(primary_color.to_string(), "--primary-color");

  let font_size = DashedIdentifier::new("--font-size-large".to_string());
  assert_eq!(font_size.to_string(), "--font-size-large");

  let spacing = DashedIdentifier::new("--spacing-sm".to_string());
  assert_eq!(spacing.to_string(), "--spacing-sm");

  // All should be valid
  assert!(DashedIdentifier::is_valid_dashed_ident("--primary-color"));
  assert!(DashedIdentifier::is_valid_dashed_ident("--font-size-large"));
  assert!(DashedIdentifier::is_valid_dashed_ident("--spacing-sm"));
}

#[test]
fn test_dashed_identifier_various_formats() {
  // Test different naming conventions that are all valid
  let kebab_case = "--kebab-case-var";
  let snake_case = "--snake_case_var";
  let camel_case = "--camelCaseVar";
  let numbers = "--var123";
  let mixed = "--my-var_123";

  assert!(DashedIdentifier::is_valid_dashed_ident(kebab_case));
  assert!(DashedIdentifier::is_valid_dashed_ident(snake_case));
  assert!(DashedIdentifier::is_valid_dashed_ident(camel_case));
  assert!(DashedIdentifier::is_valid_dashed_ident(numbers));
  assert!(DashedIdentifier::is_valid_dashed_ident(mixed));
}
