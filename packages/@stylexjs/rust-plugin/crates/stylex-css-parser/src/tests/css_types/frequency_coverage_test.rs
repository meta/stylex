use super::*;
use crate::token_types::SimpleToken;

#[test]
fn extract_frequency_token_returns_some_for_valid_hz() {
  // Happy path: Dimension token with unit "Hz" returns Some.
  let token = SimpleToken::Dimension {
    value: 440.0,
    unit: "Hz".to_string(),
  };
  let result = Frequency::extract_frequency_token(token);
  assert!(result.is_some());
  let (value, unit) = result.unwrap();
  assert_eq!(value, 440.0_f64);
  assert_eq!(unit, "Hz");
}

#[test]
fn extract_frequency_token_returns_some_for_valid_khz() {
  // Happy path: Dimension token with unit "KHz" returns Some.
  let token = SimpleToken::Dimension {
    value: 1.5,
    unit: "KHz".to_string(),
  };
  let result = Frequency::extract_frequency_token(token);
  assert!(result.is_some());
  let (value, unit) = result.unwrap();
  assert_eq!(value, 1.5_f64);
  assert_eq!(unit, "KHz");
}

#[test]
fn extract_frequency_token_returns_none_for_invalid_unit() {
  // The inner `else { None }` branch: a Dimension token with an unrecognised
  // unit (e.g. "px") yields None.
  let token = SimpleToken::Dimension {
    value: 10.0,
    unit: "px".to_string(),
  };
  assert!(Frequency::extract_frequency_token(token).is_none());
}

#[test]
fn extract_frequency_token_returns_none_for_non_dimension_token() {
  // The outer `else { None }` branch: a non-Dimension token yields None.
  // This branch is unreachable through the public parser (which guarantees a
  // Dimension token) but is now coverable via the named function.
  let token = SimpleToken::Number(440.0);
  assert!(Frequency::extract_frequency_token(token).is_none());
}
