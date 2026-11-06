use super::*;
use crate::token_types::SimpleToken;

#[test]
fn is_valid_resolution_dimension_returns_true_for_dpi() {
  // Happy path: a Dimension token with unit "dpi" returns true.
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
  // A Dimension token with an unrecognised unit returns false (inner branch).
  let token = SimpleToken::Dimension {
    value: 10.0,
    unit: "px".to_string(),
  };
  assert!(!Resolution::is_valid_resolution_dimension(&token));
}

#[test]
fn is_valid_resolution_dimension_returns_false_for_non_dimension_token() {
  // The else-branch returns false when the token is not a Dimension variant.
  // This branch is unreachable through the public parser but is now coverable
  // via the named function.
  let token = SimpleToken::Number(96.0);
  assert!(!Resolution::is_valid_resolution_dimension(&token));
}

#[test]
fn extract_dimension_token_returns_resolution_for_valid_dimension() {
  // Happy path: a proper Dimension token yields a Resolution.
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
  // The else-branch inside extract_dimension_token is unreachable through the
  // public parser. Calling the named function directly with a non-Dimension
  // token exercises that defensive branch.
  Resolution::extract_dimension_token(SimpleToken::Number(300.0));
}
