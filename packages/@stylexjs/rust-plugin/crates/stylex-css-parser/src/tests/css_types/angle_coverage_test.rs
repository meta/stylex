use super::*;
use crate::token_types::SimpleToken;

#[test]
fn is_valid_angle_dimension_returns_false_for_non_dimension_token() {
  // Unreachable through the public parser; called directly to cover the defensive branch.
  let token = SimpleToken::Number(42.0);
  assert!(!Angle::is_valid_angle_dimension(&token));
}

#[test]
#[should_panic]
fn extract_dimension_token_panics_for_non_dimension() {
  // Unreachable through the public parser; called directly to cover the defensive branch.
  Angle::extract_dimension_token(SimpleToken::Number(45.0));
}

#[test]
fn is_zero_number_returns_false_for_non_number_token() {
  // Unreachable through the public parser; called directly to cover the defensive branch.
  let token = SimpleToken::Ident("zero".to_string());
  assert!(!Angle::is_zero_number(&token));
}
