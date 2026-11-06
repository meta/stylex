use super::*;
use crate::token_types::SimpleToken;

#[test]
fn is_valid_angle_dimension_returns_true_for_valid_unit() {
  let token = SimpleToken::Dimension {
    value: 45.0,
    unit: "deg".to_string(),
  };
  assert!(Angle::is_valid_angle_dimension(&token));
}

#[test]
fn is_valid_angle_dimension_returns_false_for_invalid_unit() {
  let token = SimpleToken::Dimension {
    value: 10.0,
    unit: "px".to_string(),
  };
  assert!(!Angle::is_valid_angle_dimension(&token));
}

#[test]
fn is_valid_angle_dimension_returns_false_for_non_dimension_token() {
  // Unreachable through the public parser; called directly to cover the defensive branch.
  let token = SimpleToken::Number(42.0);
  assert!(!Angle::is_valid_angle_dimension(&token));
}

#[test]
fn extract_dimension_token_returns_angle_for_valid_dimension() {
  let token = SimpleToken::Dimension {
    value: 90.0,
    unit: "deg".to_string(),
  };
  let result = Angle::extract_dimension_token(token);
  assert_eq!(result.value, 90.0_f64);
  assert_eq!(result.unit, "deg");
}

#[test]
#[should_panic]
fn extract_dimension_token_panics_for_non_dimension() {
  // Unreachable through the public parser; called directly to cover the defensive branch.
  Angle::extract_dimension_token(SimpleToken::Number(45.0));
}

#[test]
fn is_zero_number_returns_true_for_zero() {
  let token = SimpleToken::Number(0.0);
  assert!(Angle::is_zero_number(&token));
}

#[test]
fn is_zero_number_returns_false_for_nonzero() {
  let token = SimpleToken::Number(1.0);
  assert!(!Angle::is_zero_number(&token));
}

#[test]
fn is_zero_number_returns_false_for_non_number_token() {
  // Unreachable through the public parser; called directly to cover the defensive branch.
  let token = SimpleToken::Ident("zero".to_string());
  assert!(!Angle::is_zero_number(&token));
}
