use super::*;
use crate::token_types::SimpleToken;

#[test]
fn is_valid_fr_dimension_returns_true_for_valid_fr() {
  // Happy path: a Dimension token with unit "fr" and non-negative value.
  let token = SimpleToken::Dimension {
    value: 2.5,
    unit: "fr".to_string(),
  };
  assert!(Flex::is_valid_fr_dimension(&token));
}

#[test]
fn is_valid_fr_dimension_returns_false_for_wrong_unit() {
  // A Dimension token with a unit other than "fr" returns false (inner branch).
  let token = SimpleToken::Dimension {
    value: 1.0,
    unit: "px".to_string(),
  };
  assert!(!Flex::is_valid_fr_dimension(&token));
}

#[test]
fn is_valid_fr_dimension_returns_false_for_negative_value() {
  // A Dimension token with unit "fr" but negative value returns false (inner branch).
  let token = SimpleToken::Dimension {
    value: -1.0,
    unit: "fr".to_string(),
  };
  assert!(!Flex::is_valid_fr_dimension(&token));
}

#[test]
fn is_valid_fr_dimension_returns_false_for_non_dimension_token() {
  // The else-branch returns false when the token is not a Dimension variant.
  // This branch is unreachable through the public parser but is now coverable
  // via the named function.
  let token = SimpleToken::Number(1.0);
  assert!(!Flex::is_valid_fr_dimension(&token));
}

#[test]
fn extract_dimension_token_returns_flex_for_fr_dimension() {
  // Happy path: a Dimension token with unit "fr" yields a Flex.
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
  // The else-branch inside extract_dimension_token is unreachable through the
  // public parser. Calling the named function directly with a non-Dimension
  // token exercises that defensive branch.
  Flex::extract_dimension_token(SimpleToken::Number(1.0));
}
