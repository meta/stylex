use super::*;

#[test]
fn test_flex_zero_value() {
  let zero_flex = Flex::new(0.0);
  assert_eq!(zero_flex.fraction, 0.0);
  assert_eq!(zero_flex.to_string(), "0fr");
  assert!(Flex::is_valid_fraction(0.0));
}
