#[cfg(test)]
mod round_f64_tests {
  use crate::math::round_f64;

  #[test]
  fn rounds_to_one_decimal() {
    assert_eq!(round_f64(0.30000000000000004, 1), 0.3);
    assert_eq!(round_f64(0.25, 1), 0.3);
    assert_eq!(round_f64(0.0, 1), 0.0);
  }

  #[test]
  fn rounds_to_two_decimals() {
    assert_eq!(round_f64(1.555, 2), 1.56);
  }

  #[test]
  fn rounds_to_four_decimals() {
    assert_eq!(round_f64(33.333333333333336, 4), 33.3333);
  }

  #[test]
  fn preserves_exact_values_four_decimals() {
    assert_eq!(round_f64(10.0, 4), 10.0);
  }

  #[test]
  fn rounds_negative_values() {
    // `-1.555 * 100` is exactly `-155.5`, an exact tie: JS rounds it towards
    // positive infinity, to `-155`.
    assert_eq!(round_f64(-1.555, 2), -1.55);
    assert_eq!(round_f64(-1.556, 2), -1.56);
  }

  #[test]
  fn rounds_large_values() {
    assert_eq!(round_f64(1234567.891, 2), 1234567.89);
  }

  #[test]
  fn rounds_very_small_values() {
    assert_eq!(round_f64(0.001, 2), 0.0);
  }

  #[test]
  fn rounds_to_zero_decimals() {
    assert_eq!(round_f64(3.7, 0), 4.0);
    assert_eq!(round_f64(3.2, 0), 3.0);
  }

  #[test]
  fn rounds_negative() {
    assert_eq!(round_f64(-2.345, 2), -2.35);
    // An exact negative tie breaks towards positive infinity, as Math.round does.
    assert_eq!(round_f64(-1.555, 2), -1.55);
  }

  #[test]
  fn rounds_zero() {
    assert_eq!(round_f64(0.0, 3), 0.0);
  }

  #[test]
  fn rounds_small_fractions() {
    assert_eq!(round_f64(0.0001, 3), 0.0);
    assert_eq!(round_f64(0.0005, 3), 0.001);
  }
}

#[cfg(test)]
mod js_math_round_tests {
  use crate::math::{js_math_round, round_f64};

  /// Every expectation is the value JS `Math.round` returns for the same input.
  #[test]
  fn breaks_ties_towards_positive_infinity() {
    let cases: &[(f64, f64)] = &[
      (0.5, 1.0),
      (-0.5, 0.0),
      (1.5, 2.0),
      (-1.5, -1.0),
      (2.5, 3.0),
      (-2.5, -2.0),
      (4.5, 5.0),
      (-4.5, -4.0),
    ];

    for (value, expected) in cases {
      assert_eq!(js_math_round(*value), *expected, "Math.round({})", value);
    }
  }

  #[test]
  fn rounds_nearest_when_not_a_tie() {
    assert_eq!(js_math_round(2.4), 2.0);
    assert_eq!(js_math_round(2.6), 3.0);
    assert_eq!(js_math_round(-2.4), -2.0);
    assert_eq!(js_math_round(-2.6), -3.0);
    assert_eq!(js_math_round(0.0), 0.0);
  }

  /// The largest `f64` below `0.5`. Adding `0.5` to it rounds up to exactly
  /// `1.0`, so a `(x + 0.5).floor()` implementation would answer `1` where JS
  /// answers `0`.
  #[test]
  fn rounds_down_just_below_a_tie() {
    assert_eq!(js_math_round(0.49999999999999994), 0.0);
    assert_eq!(js_math_round(-0.49999999999999994), 0.0);
  }

  #[test]
  fn negative_half_step_matches_js_at_four_decimal_places() {
    // `Math.round(-0.00005 * 10000) / 10000` is `-0`, not `-0.0001`.
    assert_eq!(round_f64(-0.00005, 4), 0.0);
    assert_eq!(round_f64(0.00005, 4), 0.0001);
    assert_eq!(round_f64(-0.000151, 4), -0.0002);
  }
}
