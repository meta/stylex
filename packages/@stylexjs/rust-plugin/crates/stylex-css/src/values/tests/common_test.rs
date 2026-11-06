use crate::values::common::{split_value, split_value_required};
use stylex_structures::raw_value::TRawValue;

// ── split_value ──────────────────────────────────────────────────────

#[test]
fn split_value_single_value() {
  let (top, right, bottom, left) = split_value(Some(&TRawValue::from("10px")));
  assert_eq!(top, "10px");
  assert!(right.is_none());
  assert!(bottom.is_none());
  assert!(left.is_none());
}

#[test]
fn split_value_two_values() {
  let (top, right, bottom, left) = split_value(Some(&TRawValue::from("10px 20px")));
  assert_eq!(top, "10px");
  assert_eq!(right.unwrap(), "20px");
  assert!(bottom.is_none());
  assert!(left.is_none());
}

#[test]
fn split_value_three_values() {
  let (top, right, bottom, left) = split_value(Some(&TRawValue::from("10px 20px 30px")));
  assert_eq!(top, "10px");
  assert_eq!(right.unwrap(), "20px");
  assert_eq!(bottom.unwrap(), "30px");
  assert!(left.is_none());
}

#[test]
fn split_value_four_values() {
  let (top, right, bottom, left) = split_value(Some(&TRawValue::from("10px 20px 30px 40px")));
  assert_eq!(top, "10px");
  assert_eq!(right.unwrap(), "20px");
  assert_eq!(bottom.unwrap(), "30px");
  assert_eq!(left.unwrap(), "40px");
}

#[test]
fn split_value_none_input() {
  let (top, right, bottom, left) = split_value(None);
  assert_eq!(top, "");
  assert!(right.is_none());
  assert!(bottom.is_none());
  assert!(left.is_none());
}

#[test]
fn split_value_empty_string() {
  let (top, right, bottom, left) = split_value(Some(&TRawValue::from("")));
  assert_eq!(top, "");
  assert!(right.is_none());
  assert!(bottom.is_none());
  assert!(left.is_none());
}

// ── split_value_required ─────────────────────────────────────────────

#[test]
fn split_value_required_single() {
  let (top, right, bottom, left) = split_value_required(Some(&TRawValue::from("10px")));
  assert_eq!(top, "10px");
  assert_eq!(right, "10px"); // falls back to top
  assert_eq!(bottom, "10px"); // falls back to top
  assert_eq!(left, "10px"); // falls back to right (which is top)
}

#[test]
fn split_value_required_two() {
  let (top, right, bottom, left) = split_value_required(Some(&TRawValue::from("10px 20px")));
  assert_eq!(top, "10px");
  assert_eq!(right, "20px");
  assert_eq!(bottom, "10px"); // falls back to top
  assert_eq!(left, "20px"); // falls back to right
}

#[test]
fn split_value_required_three() {
  let (top, right, bottom, left) = split_value_required(Some(&TRawValue::from("10px 20px 30px")));
  assert_eq!(top, "10px");
  assert_eq!(right, "20px");
  assert_eq!(bottom, "30px");
  assert_eq!(left, "20px"); // falls back to right
}

#[test]
fn split_value_required_four() {
  let (top, right, bottom, left) =
    split_value_required(Some(&TRawValue::from("10px 20px 30px 40px")));
  assert_eq!(top, "10px");
  assert_eq!(right, "20px");
  assert_eq!(bottom, "30px");
  assert_eq!(left, "40px");
}

#[test]
fn split_value_required_none() {
  let (top, right, bottom, left) = split_value_required(None);
  assert_eq!(top, "");
  assert_eq!(right, "");
  assert_eq!(bottom, "");
  assert_eq!(left, "");
}

#[test]
fn split_value_with_zero_values() {
  let (top, right, bottom, left) = split_value_required(Some(&TRawValue::from("0")));
  assert_eq!(top, "0");
  assert_eq!(right, "0");
  assert_eq!(bottom, "0");
  assert_eq!(left, "0");
}

#[test]
fn split_value_with_mixed_units() {
  let (top, right, bottom, left) = split_value_required(Some(&TRawValue::from("10px 2em")));
  assert_eq!(top, "10px");
  assert_eq!(right, "2em");
  assert_eq!(bottom, "10px");
  assert_eq!(left, "2em");
}

/// A number is handed back untouched: each expanded property appends its own
/// unit suffix, which parsing it as CSS text would lose.
#[test]
fn split_value_returns_a_number_unchanged() {
  let (top, right, bottom, left) = split_value(Some(&TRawValue::Number(10.0)));

  assert_eq!(top, TRawValue::Number(10.0));
  assert!(right.is_none());
  assert!(bottom.is_none());
  assert!(left.is_none());
}

#[test]
fn split_value_required_repeats_a_number_across_every_side() {
  let (top, right, bottom, left) = split_value_required(Some(&TRawValue::Number(10.0)));

  for side in [top, right, bottom, left] {
    assert_eq!(side, TRawValue::Number(10.0));
  }
}
