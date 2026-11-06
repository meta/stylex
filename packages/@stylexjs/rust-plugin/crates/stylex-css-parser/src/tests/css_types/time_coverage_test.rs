use super::*;
use crate::token_types::SimpleToken;

#[test]
fn extract_time_token_returns_some_for_valid_s() {
  // Happy path: Dimension token with unit "s" returns Some.
  let token = SimpleToken::Dimension {
    value: 1.5,
    unit: "s".to_string(),
  };
  let result = Time::extract_time_token(token);
  assert!(result.is_some());
  let (value, unit) = result.unwrap();
  assert_eq!(value, 1.5_f64);
  assert_eq!(unit, "s");
}

#[test]
fn extract_time_token_returns_some_for_valid_ms() {
  // Happy path: Dimension token with unit "ms" returns Some.
  let token = SimpleToken::Dimension {
    value: 500.0,
    unit: "ms".to_string(),
  };
  let result = Time::extract_time_token(token);
  assert!(result.is_some());
  let (value, unit) = result.unwrap();
  assert_eq!(value, 500.0_f64);
  assert_eq!(unit, "ms");
}

#[test]
fn extract_time_token_returns_none_for_invalid_unit() {
  // The inner `else { None }` branch: a Dimension token with an unrecognised
  // unit (e.g. "px") yields None.
  let token = SimpleToken::Dimension {
    value: 10.0,
    unit: "px".to_string(),
  };
  assert!(Time::extract_time_token(token).is_none());
}

#[test]
fn extract_time_token_returns_none_for_non_dimension_token() {
  // The outer `else { None }` branch: a non-Dimension token yields None.
  // This branch is unreachable through the public parser (which guarantees a
  // Dimension token) but is now coverable via the named function.
  let token = SimpleToken::Number(1.0);
  assert!(Time::extract_time_token(token).is_none());
}
