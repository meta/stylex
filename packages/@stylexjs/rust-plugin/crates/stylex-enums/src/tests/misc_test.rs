//! `ToNumber` of a folded binary expression.
//!
//! One reading for every caller that asks a folded binary for a number, so a
//! kind added to the enum cannot become a number in one caller and a refusal in
//! the other. Asked here rather than through a source, because the callers each
//! hand it a value they folded themselves.

use crate::misc::BinaryExprType;

/// A number answers itself, and a boolean answers the number the language
/// coerces it to -- which is what a comparison written where a number is asked
/// for reads as.
#[test]
fn a_number_and_a_boolean_each_have_a_number() {
  assert_eq!(BinaryExprType::Number(2.5).as_number(), Some(2.5));
  assert_eq!(BinaryExprType::Boolean(true).as_number(), Some(1.0));
  assert_eq!(BinaryExprType::Boolean(false).as_number(), Some(0.0));
}

/// A concatenation has a number only through `StringToNumber`, which reads the
/// text rather than the fold, so it is the caller's to ask for. A fold that
/// answered nothing has none at all.
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
