use super::*;
use crate::token_types::SimpleToken;

#[test]
fn extract_length_token_returns_some_for_dimension() {
  let token = SimpleToken::Dimension {
    value: 16.0,
    unit: "px".to_string(),
  };
  let result = Length::extract_length_token(token);
  assert!(result.is_some());
  let (value, unit) = result.unwrap();
  assert_eq!(value, 16.0_f64);
  assert_eq!(unit, "px");
}

#[test]
fn extract_length_token_returns_none_for_non_dimension_token() {
  // Unreachable through the public parser; called directly to cover the defensive branch.
  let token = SimpleToken::Number(10.0);
  assert!(Length::extract_length_token(token).is_none());
}

#[test]
fn is_valid_length_opt_returns_true_for_valid_unit() {
  let opt = Some((10.0_f64, "px".to_string()));
  assert!(Length::is_valid_length_opt(&opt));
}

#[test]
fn is_valid_length_opt_returns_false_for_invalid_unit() {
  let opt = Some((10.0_f64, "unknown".to_string()));
  assert!(!Length::is_valid_length_opt(&opt));
}

#[test]
fn is_valid_length_opt_returns_false_for_none() {
  // Unreachable through the public parser; called directly to cover the defensive branch.
  let opt: Option<(f64, String)> = None;
  assert!(!Length::is_valid_length_opt(&opt));
}

#[test]
fn is_zero_number_returns_true_for_zero() {
  let token = SimpleToken::Number(0.0);
  assert!(Length::is_zero_number(&token));
}

#[test]
fn is_zero_number_returns_false_for_nonzero() {
  let token = SimpleToken::Number(5.0);
  assert!(!Length::is_zero_number(&token));
}

#[test]
fn is_zero_number_returns_false_for_non_number_token() {
  // Unreachable through the public parser; called directly to cover the defensive branch.
  let token = SimpleToken::Ident("zero".to_string());
  assert!(!Length::is_zero_number(&token));
}
