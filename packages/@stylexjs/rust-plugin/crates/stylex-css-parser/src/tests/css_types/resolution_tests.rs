use super::*;

#[test]
fn test_resolution_units_constant() {
  let units = Resolution::units();
  assert_eq!(units.len(), 3);
  assert!(units.contains(&"dpi"));
  assert!(units.contains(&"dpcm"));
  assert!(units.contains(&"dppx"));
}
