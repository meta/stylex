/// Rounds like JS `Math.round`: to the nearest integer, and on an exact tie
/// towards positive infinity.
///
/// `f64::round` breaks ties away from zero instead, so the two disagree on
/// every negative half — `Math.round(-0.5)` is `0`, `(-0.5f64).round()` is
/// `-1`. Correcting the tie afterwards keeps `f64::round`'s exactness for
/// everything else, which a naive `(value + 0.5).floor()` would lose:
/// `0.49999999999999994 + 0.5` rounds up to `1.0` before the floor ever runs.
///
/// A tie that lands on zero yields `+0` where JS yields `-0`; the two compare
/// equal and render identically as `"0"`.
pub fn js_math_round(value: f64) -> f64 {
  let rounded = value.round();

  // Only an exact negative tie lands on the wrong side of `value`.
  if value - rounded == 0.5 {
    rounded + 1.0
  } else {
    rounded
  }
}

/// Rounds a floating-point value to the given number of decimal places.
///
/// Ties break like JS `Math.round`, towards positive infinity.
pub fn round_f64(value: f64, decimal_places: u32) -> f64 {
  let multiplier = 10f64.powi(decimal_places as i32);
  js_math_round(value * multiplier) / multiplier
}

#[cfg(test)]
#[path = "tests/math_test.rs"]
mod tests;
