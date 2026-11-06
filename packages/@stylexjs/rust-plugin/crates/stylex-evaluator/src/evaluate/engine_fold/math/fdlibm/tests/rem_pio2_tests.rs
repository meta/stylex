//! `rem_pio2` against `__ieee754_rem_pio2` of `ieee754.cc`.
//!
//! Node does not expose the reduction, so each row is the output of the C code
//! of V8, compiled without fused multiply-add. The rows of `trig_tests.rs`
//! check the same reduction against the `Math` of Node.

use super::super::test_support::assert_same_bits;
use std::f64::consts::{FRAC_PI_2, PI};

use super::rem_pio2;

/// Arguments and the `n`, the head and the tail that the C code gives.
const REFERENCE_ROWS: &[(f64, i32, f64, f64)] = &[
  // |x| < 3pi/4: 33 + 53 bits of pi/2.
  (1.0, 1, -0.5707963267948967, 4.9789962508669555e-17),
  // The same, for a negative x.
  (-1.0, -1, 0.5707963267948967, -4.9789962508669555e-17),
  // Near pi/2: 33 + 33 + 53 bits of pi/2.
  (FRAC_PI_2, 1, -6.123233995736766e-17, 1.4974857633995285e-33),
  // The same, for a negative x.
  (
    -FRAC_PI_2,
    -1,
    6.123233995736766e-17,
    -1.4974857633995285e-33,
  ),
  // n < 32 and not near n * pi/2: one pass.
  (3.0, 2, -0.14159265358979323, -1.1442377445176544e-17),
  // The same, for a negative x.
  (-3.0, -2, 0.14159265358979323, 1.1442377445176544e-17),
  // n >= 32, but no cancellation: one pass.
  (100.0, 64, -0.5309649148733836, -3.3089170858102444e-17),
  // The same, for a negative x.
  (-100.0, -64, 0.5309649148733836, 3.3089170858102444e-17),
  // Near 100 * pi/2: a second pass.
  (
    157.0796328,
    100,
    1.2051035112557021e-7,
    2.4912773691352753e-24,
  ),
  // Nearer to pi: a third pass.
  (PI, 2, -1.2246467991473532e-16, 2.994769809718341e-33),
  // Near -200000 * pi/2: a third pass.
  (
    -314159.2653589793,
    -200000,
    3.3960653996302193e-11,
    -6.809585134838895e-30,
  ),
  // A large x, with a fraction of 0.5 or more and a carry.
  (10000000.0, 6, -0.4340490172675572, -2.1635325148753377e-17),
  // The same, for a negative x.
  (-10000000.0, -6, 0.4340490172675572, 2.1635325148753377e-17),
  // A large x whose fraction needs two pieces of 24 bits.
  (1671168.0, 3, -0.6412807637183036, -2.4743163334511315e-17),
  // 2^51: a zero low piece before the carry.
  (
    2251799813685248.0,
    1,
    -0.5319107191442616,
    -3.353752670501247e-17,
  ),
  // q0 = 0.
  (
    140737488355328.0,
    2,
    0.06493035047816469,
    1.7309258282721993e-18,
  ),
  // q0 > 0 and a fraction of 0.5 or more.
  (
    316659348799488.0,
    5,
    -0.6393048748215777,
    -4.406882162483945e-17,
  ),
  // q0 > 0, and more terms of 2/pi.
  (1e22, 3, 0.5506189342358097, -7.985621383201303e-18),
  // The double nearest to a multiple of pi/2: zero pieces at the top.
  (
    5.319372648326541e255,
    5,
    4.687165924254628e-19,
    -4.372128201315243e-36,
  ),
  // A huge x.
  (1e300, 7, -0.613076157357336, 1.2100877964074999e-17),
  // The largest double, negative.
  (
    -1.7976931348623157e308,
    -2,
    0.004961975150787273,
    3.65643818040935e-19,
  ),
];

#[test]
fn rem_pio2_gives_the_reduction_of_the_c_code() {
  for &(x, n, y0, y1) in REFERENCE_ROWS {
    let (actual_n, actual_y0, actual_y1) = rem_pio2(x);
    assert_eq!(actual_n, n, "{x:?} gave n = {actual_n}, not {n}");
    assert_same_bits(actual_y0, y0, x);
    assert_same_bits(actual_y1, y1, x);
  }
}
