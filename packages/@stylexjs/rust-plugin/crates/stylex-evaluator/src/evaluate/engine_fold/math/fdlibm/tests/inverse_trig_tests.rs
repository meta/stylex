//! `asin`, `acos`, `atan` and `atan2` against the `Math` of Node.

use std::f64::consts::{FRAC_PI_2, FRAC_PI_3, FRAC_PI_4, FRAC_PI_6, PI};

use super::super::test_support::assert_same_bits;
use super::{acos, asin, atan, atan2};

/// A NaN with the exponent of an infinity and only the low bits set.
const LOW_NAN: f64 = f64::from_bits(0x7FF0_0000_0000_0001);

/// Arguments of `asin` and the answer of Node for each of them.
const ASIN_ROWS: &[(f64, f64)] = &[
  // The C library gives a different last bit for this.
  (0.5, FRAC_PI_6),
  // |x| < 0.5.
  (0.3, 0.3046926540153975),
  (-0.4, -0.41151684606748806),
  (0.1, 0.1001674211615598),
  (0.05, 0.050020856805770016),
  // |x| < 2^-27, and 2^-27 itself.
  (1e-8, 1e-8),
  (-1.5e-8, -1.5e-8),
  (-0.0, -0.0),
  (7.450580596923828e-9, 7.450580596923828e-9),
  // 0.5 <= |x| <= 0.975.
  (0.6, 0.6435011087932844),
  (-0.7, -0.775397496610753),
  (0.97, 1.3252308092796046),
  // |x| > 0.975.
  (0.975, 1.3467210414930773),
  (0.98, 1.3704614844717768),
  (-0.99, -1.4292568534704693),
  (0.9999999, 1.5703491131957876),
  // |x| = 1, and |x| > 1.
  (1.0, FRAC_PI_2),
  (-1.0, -FRAC_PI_2),
  (1.0000000000000002, f64::NAN),
  (-2.0, f64::NAN),
  (f64::INFINITY, f64::NAN),
  (f64::NAN, f64::NAN),
];

/// Arguments of `acos` and the answer of Node for each of them.
const ACOS_ROWS: &[(f64, f64)] = &[
  // The C library gives a different last bit for this.
  (0.05, 1.5207754699891267),
  // |x| < 0.5.
  (0.3, 1.2661036727794992),
  (-0.4, 1.9823131728623846),
  (1e-17, FRAC_PI_2),
  // |x| <= 2^-57.
  (6.938893903907228e-18, FRAC_PI_2),
  (-1e-20, FRAC_PI_2),
  (0.0, FRAC_PI_2),
  (-0.0, FRAC_PI_2),
  (5e-324, FRAC_PI_2),
  // x >= 0.5.
  (0.5, FRAC_PI_3),
  (0.6, 0.9272952180016123),
  (0.9, 0.45102681179626236),
  (0.9999999, 0.00044721359910904126),
  // x <= -0.5.
  (-0.5, 2.0943951023931957),
  (-0.6, 2.214297435588181),
  (-0.9, 2.6905658417935308),
  (-0.9999999, 3.141145439990684),
  // |x| = 1, and |x| > 1.
  (1.0, 0.0),
  (-1.0, PI),
  (1.0000000000000002, f64::NAN),
  (-3.0, f64::NAN),
  (f64::NEG_INFINITY, f64::NAN),
  (f64::NAN, f64::NAN),
];

/// Arguments of `atan` and the answer of Node for each of them.
const ATAN_ROWS: &[(f64, f64)] = &[
  // The C library gives a different last bit for this one. 0.5 <= |x| < 11/16.
  (0.5, 0.4636476090008061),
  // |x| < 7/16.
  (0.3, 0.2914567944778671),
  (-0.2, -0.19739555984988078),
  // |x| < 2^-27, and 2^-27 itself.
  (1e-9, 1e-9),
  (-1e-9, -1e-9),
  (-0.0, -0.0),
  (7.450580596923828e-9, 7.450580596923828e-9),
  // 7/16 <= |x| < 11/16.
  (0.4375, 0.4124104415973873),
  (0.6, 0.5404195002705842),
  (-0.6, -0.5404195002705842),
  // 11/16 <= |x| < 19/16.
  (0.7, 0.6107259643892086),
  (1.0, FRAC_PI_4),
  (-1.1, -0.8329812666744317),
  // 19/16 <= |x| < 2.4375.
  (1.2, 0.8760580505981934),
  (2.0, 1.1071487177940904),
  (-2.4, -1.1760052070951352),
  // 2.4375 <= |x| < 2^66.
  (2.4375, 1.1814796049617557),
  (10.0, 1.4711276743037347),
  (-1e10, -1.5707963266948965),
  (1e19, FRAC_PI_2),
  // |x| >= 2^66.
  (73786976294838206464.0, FRAC_PI_2),
  (-73786976294838206464.0, -FRAC_PI_2),
  (1e300, FRAC_PI_2),
  (f64::INFINITY, FRAC_PI_2),
  (f64::NEG_INFINITY, -FRAC_PI_2),
  (f64::NAN, f64::NAN),
  (LOW_NAN, f64::NAN),
];

/// Arguments `(y, x)` of `atan2` and the answer of Node for each of them.
const ATAN2_ROWS: &[((f64, f64), f64)] = &[
  // The C library gives a different last bit for this.
  ((0.08, 0.3), 0.26060239174734096),
  // The four quadrants.
  ((1.0, 2.0), 0.4636476090008061),
  ((-1.0, 2.0), -0.4636476090008061),
  ((1.0, -2.0), 2.677945044588987),
  ((-1.0, -2.0), -2.677945044588987),
  // x = 1 gives atan(y).
  ((0.5, 1.0), 0.4636476090008061),
  ((-3.0, 1.0), -1.2490457723982544),
  // y is zero.
  ((0.0, 1.0), 0.0),
  ((-0.0, 1.0), -0.0),
  ((0.0, -1.0), PI),
  ((-0.0, -1.0), -PI),
  ((0.0, 0.0), 0.0),
  ((-0.0, 0.0), -0.0),
  ((0.0, -0.0), PI),
  ((-0.0, -0.0), -PI),
  ((0.0, f64::INFINITY), 0.0),
  ((0.0, f64::NEG_INFINITY), PI),
  // x is zero.
  ((2.0, 0.0), FRAC_PI_2),
  ((-2.0, 0.0), -FRAC_PI_2),
  ((2.0, -0.0), FRAC_PI_2),
  ((-2.0, -0.0), -FRAC_PI_2),
  ((5e-324, -0.0), FRAC_PI_2),
  // x is an infinity.
  ((f64::INFINITY, f64::INFINITY), FRAC_PI_4),
  ((f64::NEG_INFINITY, f64::INFINITY), -FRAC_PI_4),
  ((f64::INFINITY, f64::NEG_INFINITY), 2.356194490192345),
  ((f64::NEG_INFINITY, f64::NEG_INFINITY), -2.356194490192345),
  ((1.0, f64::INFINITY), 0.0),
  ((-1.0, f64::INFINITY), -0.0),
  ((1.0, f64::NEG_INFINITY), PI),
  ((-1.0, f64::NEG_INFINITY), -PI),
  // y is an infinity.
  ((f64::INFINITY, 3.0), FRAC_PI_2),
  ((f64::NEG_INFINITY, 3.0), -FRAC_PI_2),
  ((f64::INFINITY, -3.0), FRAC_PI_2),
  ((f64::NEG_INFINITY, 0.0), -FRAC_PI_2),
  // |y / x| > 2^60.
  ((1e300, 1e-10), FRAC_PI_2),
  ((-1e300, -1e-10), -FRAC_PI_2),
  ((1e308, -1e-308), FRAC_PI_2),
  // x < 0 and |y| much smaller than |x|. The first two skip the atan.
  ((1e-300, -1.0), PI),
  ((-1e-300, -1.0), -PI),
  ((-5e-324, -1e-300), -PI),
  ((1e-18, -1.0), PI),
  // x > 0 and |y| much smaller than |x|.
  ((1e-300, 3.0), 3.3333333333333334e-301),
  ((1e-300, 1.0), 1e-300),
  ((-1e-300, 1.0), -1e-300),
  // NaN in either argument.
  ((f64::NAN, 1.0), f64::NAN),
  ((1.0, f64::NAN), f64::NAN),
  ((f64::NAN, f64::NAN), f64::NAN),
  ((0.0, f64::NAN), f64::NAN),
  ((f64::NAN, f64::INFINITY), f64::NAN),
  ((LOW_NAN, 1.0), f64::NAN),
  ((1.0, LOW_NAN), f64::NAN),
];

#[test]
fn asin_gives_the_bits_of_node() {
  for &(x, expected) in ASIN_ROWS {
    assert_same_bits(asin(x), expected, x);
  }
}

#[test]
fn acos_gives_the_bits_of_node() {
  for &(x, expected) in ACOS_ROWS {
    assert_same_bits(acos(x), expected, x);
  }
}

#[test]
fn atan_gives_the_bits_of_node() {
  for &(x, expected) in ATAN_ROWS {
    assert_same_bits(atan(x), expected, x);
  }
}

#[test]
fn atan2_gives_the_bits_of_node() {
  for &((y, x), expected) in ATAN2_ROWS {
    assert_same_bits(atan2(y, x), expected, (y, x));
  }
}
