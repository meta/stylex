use super::*;
use crate::token_types::SimpleToken;

#[test]
fn extract_time_token_returns_none_for_invalid_unit() {
  let token = SimpleToken::Dimension {
    value: 10.0,
    unit: "px".to_string(),
  };
  assert!(Time::extract_time_token(token).is_none());
}

#[test]
fn extract_time_token_returns_none_for_non_dimension_token() {
  // Unreachable through the public parser; called directly to cover the defensive branch.
  let token = SimpleToken::Number(1.0);
  assert!(Time::extract_time_token(token).is_none());
}
