use super::*;

// ── is_calc ───────────────────────────────────────────────────────────────

#[test]
fn is_calc_returns_true_for_calc_variant() {
  let calc_val = LengthPercentage::parser()
    .parse_to_end("calc(10px + 5%)")
    .unwrap();
  assert!(calc_val.is_calc());
}

#[test]
fn is_calc_returns_false_for_length_variant() {
  let length_val = LengthPercentage::parser().parse_to_end("10px").unwrap();
  assert!(!length_val.is_calc());
}

#[test]
fn is_calc_returns_false_for_percentage_variant() {
  let pct_val = LengthPercentage::parser().parse_to_end("50%").unwrap();
  assert!(!pct_val.is_calc());
}

// ── as_calc ───────────────────────────────────────────────────────────────

#[test]
fn as_calc_returns_some_for_calc_variant() {
  let calc_val = LengthPercentage::parser()
    .parse_to_end("calc(100% - 20px)")
    .unwrap();
  let calc_ref = calc_val.as_calc();
  assert!(calc_ref.is_some());
  // The Calc Display includes "calc(" prefix.
  assert!(calc_ref.unwrap().to_string().starts_with("calc("));
}

#[test]
fn as_calc_returns_none_for_length_variant() {
  let length_val = LengthPercentage::parser().parse_to_end("16px").unwrap();
  assert_eq!(length_val.as_calc(), None);
}

#[test]
fn as_calc_returns_none_for_percentage_variant() {
  let pct_val = LengthPercentage::parser().parse_to_end("25%").unwrap();
  assert_eq!(pct_val.as_calc(), None);
}
