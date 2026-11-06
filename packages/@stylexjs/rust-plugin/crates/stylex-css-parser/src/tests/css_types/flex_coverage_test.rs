use super::*;
use crate::token_types::SimpleToken;

#[test]
fn is_valid_fr_dimension_returns_true_for_valid_fr() {
  let token = SimpleToken::Dimension {
    value: 2.5,
    unit: "fr".to_string(),
  };
  assert!(Flex::is_valid_fr_dimension(&token));
}

#[test]
fn is_valid_fr_dimension_returns_false_for_wrong_unit() {
  let token = SimpleToken::Dimension {
    value: 1.0,
    unit: "px".to_string(),
  };
  assert!(!Flex::is_valid_fr_dimension(&token));
}

#[test]
fn is_valid_fr_dimension_returns_false_for_negative_value() {
  let token = SimpleToken::Dimension {
    value: -1.0,
    unit: "fr".to_string(),
  };
  assert!(!Flex::is_valid_fr_dimension(&token));
}

#[test]
fn is_valid_fr_dimension_returns_false_for_non_dimension_token() {
  // Unreachable through the public parser; called directly to cover the defensive branch.
  let token = SimpleToken::Number(1.0);
  assert!(!Flex::is_valid_fr_dimension(&token));
}

#[test]
fn extract_dimension_token_returns_flex_for_fr_dimension() {
  let token = SimpleToken::Dimension {
    value: 3.0,
    unit: "fr".to_string(),
  };
  let result = Flex::extract_dimension_token(token);
  assert_eq!(result.fraction, 3.0_f64);
}

#[test]
#[should_panic]
fn extract_dimension_token_panics_for_non_dimension() {
  // Unreachable through the public parser; called directly to cover the defensive branch.
  Flex::extract_dimension_token(SimpleToken::Number(1.0));
}
