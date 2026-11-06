use super::*;
use crate::token_types::SimpleToken;

#[test]
fn extract_frequency_token_returns_some_for_valid_hz() {
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
  let token = SimpleToken::Dimension {
    value: 10.0,
    unit: "px".to_string(),
  };
  assert!(Frequency::extract_frequency_token(token).is_none());
}

#[test]
fn extract_frequency_token_returns_none_for_non_dimension_token() {
  // Unreachable through the public parser; called directly to cover the defensive branch.
  let token = SimpleToken::Number(440.0);
  assert!(Frequency::extract_frequency_token(token).is_none());
}
