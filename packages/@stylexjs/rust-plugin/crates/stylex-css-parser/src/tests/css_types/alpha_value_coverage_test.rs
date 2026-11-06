use super::*;
use crate::token_types::SimpleToken;

#[test]
fn extract_percentage_token_returns_value_for_percentage() {
  let token = SimpleToken::Percentage(75.0);
  let result = AlphaValue::extract_percentage_token(token);
  assert_eq!(result.value, 0.75_f64);
}

#[test]
#[should_panic]
fn extract_percentage_token_panics_for_non_percentage() {
  // Unreachable through the public parser; called directly to cover the defensive branch.
  AlphaValue::extract_percentage_token(SimpleToken::Number(0.5));
}

#[test]
#[should_panic]
fn extract_number_token_panics_for_non_number() {
  // Unreachable through the public parser; called directly to cover the defensive branch.
  AlphaValue::extract_number_token(SimpleToken::Percentage(0.5));
}

// ── parse_alpha_token ──
