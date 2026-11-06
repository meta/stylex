use super::*;

#[test]
fn test_angle_units_constant() {
  let units = Angle::units();
  assert_eq!(units.len(), 4);
  assert!(units.contains(&"deg"));
  assert!(units.contains(&"grad"));
  assert!(units.contains(&"rad"));
  assert!(units.contains(&"turn"));
}
