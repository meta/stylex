//! `sinh`, `cosh`, `tanh`, `asinh`, `acosh`, `atanh` against the `Math` of Node.

// The answers of Node are near the constants of `std` on purpose. A row
// tests a branch, so it keeps the number that Node prints.

use super::super::test_support::assert_same_bits;
use super::{acosh, asinh, atanh, cosh, sinh, tanh};

/// Arguments and the answer of `Math.sinh` in Node for each of them. They test |x| near 2^-28,
/// 1, 22, ln(MAX_VALUE) and the overflow limit.
const SINH_ROWS: &[(f64, f64)] = &[
  (f64::NAN, f64::NAN),
  (f64::INFINITY, f64::INFINITY),
  (f64::NEG_INFINITY, f64::NEG_INFINITY),
  (0.0, 0.0),
  (5e-324, 5e-324),
  (-5e-324, -5e-324),
  (2.2250738585072014e-308, 2.2250738585072014e-308),
  (-0.0, -0.0),
  (1e-10, 1e-10),
  (3.725290298461914e-9, 3.725290298461914e-9),
  (3.7252902984619136e-9, 3.7252902984619136e-9),
  (0.2, 0.20133600254109402),
  (-0.5, -0.5210953054937474),
  (1.0, 1.1752011936438014),
  (0.9999999999999999, 1.1752011936438014),
  (5.0, 74.20321057778875),
  (-21.9, -1621881641.788824),
  (22.0, 1792456423.065796),
  (21.999999999999996, 1792456423.0657895),
  (100.0, 1.3440585709080678e+43),
  (-700.0, -5.0711602736750225e+303),
  (709.7822265625, 8.984095368647863e+307),
  (709.7822265624999, 8.984095368646841e+307),
  (710.0, 1.1169973830808557e+308),
  (710.4758600739439, 1.7976931348621744e+308),
  (710.475860073944, f64::INFINITY),
  (-711.0, f64::NEG_INFINITY),
];

/// Arguments and the answer of `Math.cosh` in Node for each of them. They test |x| near 2^-55,
/// ln(2)/2, 22, ln(MAX_VALUE) and the overflow limit.
const COSH_ROWS: &[(f64, f64)] = &[
  (f64::NAN, f64::NAN),
  (f64::INFINITY, f64::INFINITY),
  (f64::NEG_INFINITY, f64::INFINITY),
  (0.0, 1.0),
  (5e-324, 1.0),
  (-5e-324, 1.0),
  (2.2250738585072014e-308, 1.0),
  (-0.0, 1.0),
  (1e-17, 1.0),
  (2.7755575615628914e-17, 1.0),
  (2.775557561562891e-17, 1.0),
  (0.2, 1.020066755619076),
  (0.4, 1.081072371838455),
  (0.34657359027997264, 1.0606601717798212),
  (0.3465735902799727, 1.0606601717798214),
  (1.0, 1.5430806348152437),
  (21.9, 1621881641.788824),
  (22.0, 1792456423.065796),
  (-100.0, 1.3440585709080678e+43),
  (709.0, 4.109203730777486e+307),
  (709.79, 9.054204815152345e+307),
  (710.4758600739439, 1.7976931348621744e+308),
  (-710.4758600739439, 1.7976931348621744e+308),
  (710.475860073944, f64::INFINITY),
  (711.0, f64::INFINITY),
];

/// Arguments and the answer of `Math.tanh` in Node for each of them. They test |x| near 2^-28,
/// 1 and 22, and both infinities.
const TANH_ROWS: &[(f64, f64)] = &[
  (f64::NAN, f64::NAN),
  (f64::INFINITY, 1.0),
  (f64::NEG_INFINITY, -1.0),
  (0.0, 0.0),
  (5e-324, 5e-324),
  (-5e-324, -5e-324),
  (2.2250738585072014e-308, 2.2250738585072014e-308),
  (-0.0, -0.0),
  (1e-10, 1e-10),
  (-1e-10, -1e-10),
  (3.725290298461914e-9, 3.725290298461914e-9),
  (0.02, 0.01999733375993093),
  (0.5, 0.46211715726000974),
  (-0.5, -0.46211715726000974),
  (0.9999999999999999, 0.7615941559557649),
  (1.0, 0.7615941559557649),
  (2.0, 0.9640275800758169),
  (-5.0, -0.9999092042625951),
  (21.9, 1.0),
  (22.0, 1.0),
  (100.0, 1.0),
  (-100.0, -1.0),
  (-f64::NAN, f64::NAN),
];

/// Arguments and the answer of `Math.asinh` in Node for each of them. They test |x| near 2^-28,
/// 2 and 2^28.
const ASINH_ROWS: &[(f64, f64)] = &[
  (f64::NAN, f64::NAN),
  (f64::INFINITY, f64::INFINITY),
  (f64::NEG_INFINITY, f64::NEG_INFINITY),
  (0.0, 0.0),
  (5e-324, 5e-324),
  (-5e-324, -5e-324),
  (2.2250738585072014e-308, 2.2250738585072014e-308),
  (-0.0, -0.0),
  (1e-10, 1e-10),
  (-1e-10, -1e-10),
  (3.725290298461914e-9, 3.725290298461914e-9),
  (0.08, 0.07991491149449678),
  (1.0, 0.881373587019543),
  (-1.5, -1.1947632172871094),
  (2.0, 1.4436354751788103),
  (2.0000000000000004, 1.4436354751788105),
  (3.0, 1.8184464592320668),
  (-100.0, -5.298342365610589),
  (268435456.0, 20.101268236238415),
  (268435456.00000006, 20.101268236238415),
  (10000000000.0, 23.7189981105004),
  (-1e+300, -691.4686750787736),
];

/// Arguments and the answer of `Math.acosh` in Node for each of them. They test x below 1, x of
/// 1, and x near 2 and 2^28.
const ACOSH_ROWS: &[(f64, f64)] = &[
  (0.5, f64::NAN),
  (-1.0, f64::NAN),
  (f64::NEG_INFINITY, f64::NAN),
  (f64::NAN, f64::NAN),
  (f64::INFINITY, f64::INFINITY),
  (1.0, 0.0),
  (1.0000000000000002, 2.1073424255447017e-8),
  (1.1, 0.4435682543851154),
  (1.5, 0.9624236501192069),
  (2.0, 1.3169578969248166),
  (2.0000000000000004, 1.316957896924817),
  (3.0, 1.7627471740390859),
  (100000.0, 12.206072645505174),
  (268435455.99999997, 20.101268236238415),
  (268435456.0, 20.10126823623841),
  (1e+300, 691.4686750787736),
  (0.0, f64::NAN),
  (5e-324, f64::NAN),
  (-5e-324, f64::NAN),
  (2.2250738585072014e-308, f64::NAN),
  (-0.0, f64::NAN),
  (0.9999999999999999, f64::NAN),
];

/// Arguments and the answer of `Math.atanh` in Node for each of them. They test |x| of 1 and
/// just above it, |x| near 2^-28 and 0.5.
const ATANH_ROWS: &[(f64, f64)] = &[
  (1.0, f64::INFINITY),
  (-1.0, f64::NEG_INFINITY),
  (1.0000000000000002, f64::NAN),
  (-1.0000000000000002, f64::NAN),
  (2.0, f64::NAN),
  (f64::NAN, f64::NAN),
  (f64::NEG_INFINITY, f64::NAN),
  (f64::INFINITY, f64::NAN),
  (0.0, 0.0),
  (5e-324, 5e-324),
  (-5e-324, -5e-324),
  (2.2250738585072014e-308, 2.2250738585072014e-308),
  (-0.0, -0.0),
  (1e-10, 1e-10),
  (-1e-10, -1e-10),
  (3.725290298461914e-9, 3.725290298461914e-9),
  (0.25, 0.25541281188299536),
  (-0.25, -0.25541281188299536),
  (0.5, 0.5493061443340548),
  (-0.5, -0.5493061443340548),
  (0.4999999999999999, 0.5493061443340547),
  (-0.9, -1.4722194895832204),
  (0.9999999999999999, 18.714973875118524),
];

#[test]
fn sinh_gives_the_bits_of_node() {
  for &(x, expected) in SINH_ROWS {
    assert_same_bits(sinh(x), expected, x);
  }
}

#[test]
fn cosh_gives_the_bits_of_node() {
  for &(x, expected) in COSH_ROWS {
    assert_same_bits(cosh(x), expected, x);
  }
}

#[test]
fn tanh_gives_the_bits_of_node() {
  for &(x, expected) in TANH_ROWS {
    assert_same_bits(tanh(x), expected, x);
  }
}

#[test]
fn asinh_gives_the_bits_of_node() {
  for &(x, expected) in ASINH_ROWS {
    assert_same_bits(asinh(x), expected, x);
  }
}

#[test]
fn acosh_gives_the_bits_of_node() {
  for &(x, expected) in ACOSH_ROWS {
    assert_same_bits(acosh(x), expected, x);
  }
}

#[test]
fn atanh_gives_the_bits_of_node() {
  for &(x, expected) in ATANH_ROWS {
    assert_same_bits(atanh(x), expected, x);
  }
}
