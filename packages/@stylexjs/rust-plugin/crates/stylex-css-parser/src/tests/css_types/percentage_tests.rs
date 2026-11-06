// Tests extracted for css_types/percentage.rs behaviors and edge cases.
// Source: crates/stylex-css-parser/src/css_types/percentage.rs

use super::*;

#[test]
fn test_percentage_reexport() {
  let pct = Percentage::new(50.0);
  assert_eq!(pct.value, 50.0);
  assert_eq!(pct.to_string(), "50%");
}

#[test]
fn test_css_percentage_alias() {
  let pct = CssPercentage::new(75.0);
  assert_eq!(pct.value, 75.0);
}
