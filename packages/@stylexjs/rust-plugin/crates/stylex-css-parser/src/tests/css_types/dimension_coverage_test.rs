use super::*;
use crate::token_types::SimpleToken;

#[test]
fn extract_dimension_token_returns_some_for_valid_unit() {
  let token = SimpleToken::Dimension {
    value: 16.0,
    unit: "px".to_string(),
  };
  let result = Dimension::extract_dimension_token(token);
  assert!(result.is_some());
  if let Some(Dimension::Length(length)) = result {
    assert_eq!(length.unit, "px");
  } else {
    panic!("Expected Dimension::Length");
  }
}

#[test]
fn extract_dimension_token_returns_none_for_unknown_unit() {
  let token = SimpleToken::Dimension {
    value: 10.0,
    unit: "unknown".to_string(),
  };
  assert!(Dimension::extract_dimension_token(token).is_none());
}

#[test]
fn extract_dimension_token_returns_none_for_non_dimension_token() {
  // Unreachable through the public parser; called directly to cover the defensive branch.
  let token = SimpleToken::Number(42.0);
  assert!(Dimension::extract_dimension_token(token).is_none());
}
