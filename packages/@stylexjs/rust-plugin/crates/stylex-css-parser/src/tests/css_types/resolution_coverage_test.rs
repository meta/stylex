use super::*;
use crate::token_types::SimpleToken;

#[test]
fn is_valid_resolution_dimension_returns_true_for_dpi() {
  let token = SimpleToken::Dimension {
    value: 96.0,
    unit: "dpi".to_string(),
  };
  assert!(Resolution::is_valid_resolution_dimension(&token));
}

#[test]
fn is_valid_resolution_dimension_returns_true_for_dpcm() {
  let token = SimpleToken::Dimension {
    value: 37.8,
    unit: "dpcm".to_string(),
  };
  assert!(Resolution::is_valid_resolution_dimension(&token));
}

#[test]
fn is_valid_resolution_dimension_returns_true_for_dppx() {
  let token = SimpleToken::Dimension {
    value: 2.0,
    unit: "dppx".to_string(),
  };
  assert!(Resolution::is_valid_resolution_dimension(&token));
}

#[test]
fn is_valid_resolution_dimension_returns_false_for_invalid_unit() {
  let token = SimpleToken::Dimension {
    value: 10.0,
    unit: "px".to_string(),
  };
  assert!(!Resolution::is_valid_resolution_dimension(&token));
}

#[test]
fn is_valid_resolution_dimension_returns_false_for_non_dimension_token() {
  // Unreachable through the public parser; called directly to cover the defensive branch.
  let token = SimpleToken::Number(96.0);
  assert!(!Resolution::is_valid_resolution_dimension(&token));
}

#[test]
fn extract_dimension_token_returns_resolution_for_valid_dimension() {
  let token = SimpleToken::Dimension {
    value: 300.0,
    unit: "dpi".to_string(),
  };
  let result = Resolution::extract_dimension_token(token);
  assert_eq!(result.value, 300.0_f64);
  assert_eq!(result.unit, "dpi");
}

#[test]
#[should_panic]
fn extract_dimension_token_panics_for_non_dimension() {
  // Unreachable through the public parser; called directly to cover the defensive branch.
  Resolution::extract_dimension_token(SimpleToken::Number(300.0));
}
