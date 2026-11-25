//! Tests for user-facing message generation functions (argument length, static
//! values, etc).

use crate::constants::messages::*;

#[test]
fn test_non_static_value_with_empty_api_name_falls_back_to_a() {
  assert_eq!(
    non_static_value(""),
    "Only static values are allowed inside of a () call."
  );
}

#[test]
fn test_type_asserted_call_value() {
  assert_eq!(
    type_asserted_call_value("create"),
    "create() cannot be written inside a type assertion. Bind the call to a variable and assert the variable."
  );
}

#[test]
fn test_expected_call_expression() {
  let result = expected_call_expression("defineVars");
  assert!(result.starts_with("defineVars(): Expected a call expression"));
}
