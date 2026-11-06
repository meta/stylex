//! `ToNumber` of a folded binary expression.

use crate::misc::BinaryExprType;

/// A boolean answers the number JS coerces it to.
#[test]
fn a_number_and_a_boolean_each_have_a_number() {
  assert_eq!(BinaryExprType::Number(2.5).as_number(), Some(2.5));
  assert_eq!(BinaryExprType::Boolean(true).as_number(), Some(1.0));
  assert_eq!(BinaryExprType::Boolean(false).as_number(), Some(0.0));
}

/// Concatenations and empty folds have no number here.
#[test]
fn a_string_and_a_refusal_have_none() {
  assert_eq!(
    BinaryExprType::String {
      text: String::from("10"),
      units: 2,
    }
    .as_number(),
    None
  );
  assert_eq!(BinaryExprType::Null.as_number(), None);
}
