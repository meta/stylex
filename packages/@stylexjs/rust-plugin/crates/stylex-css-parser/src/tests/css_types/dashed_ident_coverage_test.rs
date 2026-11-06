use super::*;
use crate::token_types::SimpleToken;

#[test]
fn extract_ident_token_returns_value_for_ident() {
  // Happy path: a proper Ident token yields its string value.
  let token = SimpleToken::Ident("--custom-prop".to_string());
  assert_eq!(
    DashedIdentifier::extract_ident_token(token),
    "--custom-prop"
  );
}

#[test]
#[should_panic]
fn extract_ident_token_panics_for_non_ident() {
  // The else-branch inside extract_ident_token is unreachable through the
  // public parser. Calling the named function directly with a non-Ident token
  // exercises that defensive branch.
  DashedIdentifier::extract_ident_token(SimpleToken::Comma);
}
