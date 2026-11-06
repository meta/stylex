//! Tests for time-unit-related helpers derived from numeric suffix metadata.

use crate::constants::time_units::get_time_units;

/// `get_time_units` should return the known time-based property keys.
#[test]
fn get_time_units_returns_expected_keys() {
  let units = get_time_units();

  assert_eq!(units.len(), 5);
  assert!(units.contains("animationDelay"));
  assert!(units.contains("animationDuration"));
  assert!(units.contains("transitionDelay"));
  assert!(units.contains("transitionDuration"));
  assert!(units.contains("voiceDuration"));
}
