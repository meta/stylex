/// Rounds like JS `Math.round`, breaking exact ties towards positive infinity.
/// A tie that lands on zero yields `+0` where JS yields `-0`; both render as `"0"`.
pub fn js_math_round(value: f64) -> f64 {
  let rounded = value.round();

  // `f64::round` breaks ties away from zero, so only exact negative ties need correcting.
  if value - rounded == 0.5 {
    rounded + 1.0
  } else {
    rounded
  }
}

/// Rounds to `decimal_places` decimal places, breaking ties like JS `Math.round`.
pub fn round_f64(value: f64, decimal_places: u32) -> f64 {
  let multiplier = 10f64.powi(decimal_places as i32);
  js_math_round(value * multiplier) / multiplier
}

#[cfg(test)]
#[path = "tests/math_test.rs"]
mod tests;
