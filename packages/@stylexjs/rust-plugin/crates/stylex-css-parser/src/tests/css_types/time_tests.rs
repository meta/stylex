use super::*;

#[test]
fn test_time_units_constant() {
  let units = Time::units();
  assert_eq!(units.len(), 2);
  assert!(units.contains(&"s"));
  assert!(units.contains(&"ms"));
}
