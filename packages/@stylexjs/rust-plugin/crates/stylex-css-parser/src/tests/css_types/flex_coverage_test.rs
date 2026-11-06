use super::*;
use crate::token_types::SimpleToken;

#[test]
fn is_valid_fr_dimension_returns_false_for_non_dimension_token() {
  // Unreachable through the public parser; called directly to cover the defensive branch.
  let token = SimpleToken::Number(1.0);
  assert!(!Flex::is_valid_fr_dimension(&token));
}

#[test]
#[should_panic]
fn extract_dimension_token_panics_for_non_dimension() {
  // Unreachable through the public parser; called directly to cover the defensive branch.
  Flex::extract_dimension_token(SimpleToken::Number(1.0));
}
