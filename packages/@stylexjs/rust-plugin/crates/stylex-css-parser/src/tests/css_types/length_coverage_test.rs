use super::*;
use crate::token_types::SimpleToken;

#[test]
fn extract_length_token_returns_some_for_dimension() {
  // Happy path: a Dimension token yields Some.
  let token = SimpleToken::Dimension {
    value: 16.0,
    unit: "px".to_string(),
  };
  let result = Length::extract_length_token(token);
  assert!(result.is_some());
  let (value, unit) = result.unwrap();
  assert_eq!(value, 16.0_f64);
  assert_eq!(unit, "px");
}

#[test]
fn extract_length_token_returns_none_for_non_dimension_token() {
  // The else-branch returns None when the token is not a Dimension variant.
  // This branch is unreachable through the public parser but is now coverable
  // via the named function.
  let token = SimpleToken::Number(10.0);
  assert!(Length::extract_length_token(token).is_none());
}

#[test]
fn is_valid_length_opt_returns_true_for_valid_unit() {
  // Happy path: Some with a recognised length unit returns true.
  let opt = Some((10.0_f64, "px".to_string()));
  assert!(Length::is_valid_length_opt(&opt));
}

#[test]
fn is_valid_length_opt_returns_false_for_invalid_unit() {
  // Some with an unrecognised unit returns false (inner branch, unit check fails).
  let opt = Some((10.0_f64, "unknown".to_string()));
  assert!(!Length::is_valid_length_opt(&opt));
}

#[test]
fn is_valid_length_opt_returns_false_for_none() {
  // The else-branch returns false when opt is None. This is unreachable through
  // the public parser (extract_length_token only returns None for non-Dimension
  // tokens, which the combinator excludes) but is coverable via direct call.
  let opt: Option<(f64, String)> = None;
  assert!(!Length::is_valid_length_opt(&opt));
}

#[test]
fn is_zero_number_returns_true_for_zero() {
  // Happy path: Number(0.0) is accepted as a zero-length.
  let token = SimpleToken::Number(0.0);
  assert!(Length::is_zero_number(&token));
}

#[test]
fn is_zero_number_returns_false_for_nonzero() {
  // Non-zero Number returns false (inner branch).
  let token = SimpleToken::Number(5.0);
  assert!(!Length::is_zero_number(&token));
}

#[test]
fn is_zero_number_returns_false_for_non_number_token() {
  // The else-branch returns false when the token is not a Number variant.
  // This branch is unreachable through the public zero_parser but is now
  // coverable via the named function.
  let token = SimpleToken::Ident("zero".to_string());
  assert!(!Length::is_zero_number(&token));
}
