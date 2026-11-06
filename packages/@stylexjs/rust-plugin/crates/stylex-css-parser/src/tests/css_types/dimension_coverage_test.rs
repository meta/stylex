use super::*;
use crate::token_types::SimpleToken;

#[test]
fn extract_dimension_token_returns_some_for_valid_unit() {
  // Happy path: a proper Dimension token with a known unit yields Some.
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
  // A Dimension token with an unrecognised unit returns None (from_value_and_unit).
  let token = SimpleToken::Dimension {
    value: 10.0,
    unit: "unknown".to_string(),
  };
  assert!(Dimension::extract_dimension_token(token).is_none());
}

#[test]
fn extract_dimension_token_returns_none_for_non_dimension_token() {
  // The else-branch returns None when the token is not a Dimension variant.
  // This branch is unreachable through the public parser but is now coverable
  // via the named function.
  let token = SimpleToken::Number(42.0);
  assert!(Dimension::extract_dimension_token(token).is_none());
}
