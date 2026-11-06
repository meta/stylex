use super::*;
use crate::token_types::SimpleToken;

#[test]
fn extract_ident_token_returns_value_for_ident() {
  let token = SimpleToken::Ident("myIdentifier".to_string());
  assert_eq!(CustomIdentifier::extract_ident_token(token), "myIdentifier");
}

#[test]
#[should_panic]
fn extract_ident_token_panics_for_non_ident() {
  // Unreachable through the public parser; called directly to cover the defensive branch.
  CustomIdentifier::extract_ident_token(SimpleToken::Number(1.0));
}
