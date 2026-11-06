use super::*;
use crate::token_types::SimpleToken;

#[test]
#[should_panic]
fn extract_ident_token_panics_for_non_ident() {
  // Unreachable through the public parser; called directly to cover the defensive branch.
  BlendMode::extract_ident_token(SimpleToken::Number(42.0));
}
