//! `cbrt` against the `Math.cbrt` of Node.

use super::super::test_support::assert_same_bits;
use super::cbrt;

/// Arguments and the answer of Node for each of them.
const REFERENCE_ROWS: &[(f64, f64)] = &[
  // The C library gives a different last bit for these.
  (0.02, 0.27144176165949063),
  (0.16, 0.5428835233189813),
  (0.18, 0.564621617328617),
  // Exact cubes.
  (27.0, 3.0),
  (-8.0, -2.0),
  (1e-300, 1e-100),
  (0.001, 0.1),
  (2.0, 1.2599210498948732),
  (3.0, 1.4422495703074083),
  (-123.456, -4.979327984674048),
  (1e308, 4.641588833612779e102),
  // The smallest normal number, and subnormal numbers of both signs.
  (2.2250738585072014e-308, 2.812644285236262e-103),
  (-1e-310, -4.641588833612774e-104),
  (5e-324, 1.7031839360032603e-108),
];

#[test]
fn cbrt_gives_the_bits_of_node() {
  for &(x, expected) in REFERENCE_ROWS {
    assert_same_bits(cbrt(x), expected, x);
  }
}

#[test]
fn cbrt_keeps_zeros_infinities_and_nan() {
  for x in [0.0, -0.0, f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
    assert_same_bits(cbrt(x), x, x);
  }
}
