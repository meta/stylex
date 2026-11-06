use super::*;

#[test]
fn test_frequency_units_constant() {
  let units = Frequency::units();
  assert_eq!(units.len(), 2);
  assert!(units.contains(&"Hz"));
  assert!(units.contains(&"KHz"));
}
