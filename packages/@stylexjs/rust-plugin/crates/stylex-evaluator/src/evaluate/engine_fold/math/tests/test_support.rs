//! The assertion that the test of each fdlibm port uses.

use std::fmt::Debug;

/// Asserts that `actual` is the same number as `expected`, to the last bit and
/// with the sign of a zero. Every `NaN` is the same number, because JavaScript
/// cannot see the bits of a `NaN`.
#[track_caller]
pub(super) fn assert_same_bits(actual: f64, expected: f64, arguments: impl Debug) {
  if expected.is_nan() {
    assert!(actual.is_nan(), "{arguments:?} gave {actual:?}, not NaN");
  } else {
    assert_eq!(
      actual.to_bits(),
      expected.to_bits(),
      "{arguments:?} gave {actual:?}, not {expected:?}"
    );
  }
}
