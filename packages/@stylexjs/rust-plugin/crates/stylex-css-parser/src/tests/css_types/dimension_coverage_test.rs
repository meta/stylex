use super::*;
use crate::token_types::SimpleToken;

#[test]
fn extract_dimension_token_returns_none_for_non_dimension_token() {
  // Unreachable through the public parser; called directly to cover the defensive branch.
  let token = SimpleToken::Number(42.0);
  assert!(Dimension::extract_dimension_token(token).is_none());
}
