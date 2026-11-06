use super::*;
use crate::token_types::SimpleToken;

#[test]
fn is_valid_angle_dimension_returns_true_for_valid_unit() {
  // Happy path: a Dimension token with a known angle unit returns true.
  let token = SimpleToken::Dimension {
    value: 45.0,
    unit: "deg".to_string(),
  };
  assert!(Angle::is_valid_angle_dimension(&token));
}

#[test]
fn is_valid_angle_dimension_returns_false_for_invalid_unit() {
  // A Dimension token with an unrecognised unit returns false (inner branch).
  let token = SimpleToken::Dimension {
    value: 10.0,
    unit: "px".to_string(),
  };
  assert!(!Angle::is_valid_angle_dimension(&token));
}

#[test]
fn is_valid_angle_dimension_returns_false_for_non_dimension_token() {
  // The else-branch returns false when the token is not a Dimension variant.
  // This branch is unreachable through the public parser but is now coverable
  // via the named function.
  let token = SimpleToken::Number(42.0);
  assert!(!Angle::is_valid_angle_dimension(&token));
}

#[test]
fn extract_dimension_token_returns_angle_for_valid_dimension() {
  // Happy path: a proper Dimension token with an angle unit yields an Angle.
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
  // The else-branch inside extract_dimension_token is unreachable through the
  // public parser. Calling the named function directly with a non-Dimension
  // token exercises that defensive branch.
  Angle::extract_dimension_token(SimpleToken::Number(45.0));
}

#[test]
fn is_zero_number_returns_true_for_zero() {
  // Happy path: Number(0.0) is the zero case.
  let token = SimpleToken::Number(0.0);
  assert!(Angle::is_zero_number(&token));
}

#[test]
fn is_zero_number_returns_false_for_nonzero() {
  // Non-zero Number returns false (inner branch).
  let token = SimpleToken::Number(1.0);
  assert!(!Angle::is_zero_number(&token));
}

#[test]
fn is_zero_number_returns_false_for_non_number_token() {
  // The else-branch returns false when the token is not a Number variant.
  // This branch is unreachable through the public zero_parser but is now
  // coverable via the named function.
  let token = SimpleToken::Ident("zero".to_string());
  assert!(!Angle::is_zero_number(&token));
}
