//! `exp`, `expm1`, `log`, `log1p`, `log2`, `log10` against the `Math` of Node.

use std::f64::consts::{E, FRAC_1_SQRT_2, LN_2, LN_10, LOG2_10, LOG10_2, SQRT_2};

use super::super::test_support::assert_same_bits;
use super::{exp, expm1, log, log1p, log2, log10};

/// Arguments and the answer of `Math.exp` in Node for each of them. They test the overflow and
/// underflow limits, the reductions of 1 and of k ln(2), the k of 1024 and below -1021, and |x|
/// near 2^-28.
// One answer of Node is one bit below `SQRT_2`, so it stays a literal.
#[allow(clippy::approx_constant)]
const EXP_ROWS: &[(f64, f64)] = &[
  (f64::NAN, f64::NAN),
  (f64::INFINITY, f64::INFINITY),
  (f64::NEG_INFINITY, 0.0),
  (709.782712893384, 1.7976931348622732e+308),
  (709.7827128933841, f64::INFINITY),
  (710.0, f64::INFINITY),
  (-745.1332191019411, 5e-324),
  (-745.1332191019412, 0.0),
  (-746.0, 0.0),
  (-720.0, 2.0322308024e-313),
  (-740.0, 4.2e-322),
  (-709.0, 1.216780750623423e-308),
  (0.5, 1.6487212707001282),
  (-0.5, 0.6065306597126334),
  (1.0, E),
  (2.0, 7.38905609893065),
  (-2.0, 0.1353352832366127),
  (10.0, 22026.465794806718),
  (-10.0, 0.00004539992976248485),
  (100.0, 2.6881171418161356e+43),
  (709.7, 1.6549840276802644e+308),
  (0.27, 1.3099644507332475),
  (-0.3, 0.7408182206817179),
  (0.1, 1.1051709180756477),
  (1e-10, 1.0000000001),
  (0.0, 1.0),
  (-0.0, 1.0),
  (1e-300, 1.0),
  (3.725290298461914e-9, 1.0000000037252903),
  (-3.725290298461914e-9, 0.9999999962747097),
  (3.7252902984619136e-9, 1.0000000037252903),
  (0.34657359027997264, 1.414213562373095),
  (0.3465735902799727, SQRT_2),
  (1.0397207708399179, 2.82842712474619),
  (1.039720770839918, 2.8284271247461907),
  (-1.0, 0.36787944117144233),
];

/// Arguments and the answer of `Math.expm1` in Node for each of them. They test the limit of 56
/// ln(2), the overflow limit, k of -1, 1, 2 to 19, 20 to 56, above 56 and 1024, and |x| near
/// 2^-54.
const EXPM1_ROWS: &[(f64, f64)] = &[
  (f64::NAN, f64::NAN),
  (f64::INFINITY, f64::INFINITY),
  (f64::NEG_INFINITY, -1.0),
  (709.782712893384, 1.7976931348622732e+308),
  (709.7827128933841, f64::INFINITY),
  (710.0, f64::INFINITY),
  (-40.0, -1.0),
  (-38.9, -1.0),
  (-1000.0, -1.0),
  (-710.0, -1.0),
  (40.0, 235385266837019970.0),
  (50.0, 5.184705528587072e+21),
  (0.5, 0.6487212707001282),
  (-0.5, -0.3934693402873666),
  (0.4, 0.49182469764127035),
  (0.8, 1.2255409284924677),
  (-0.4, -0.32967995396436073),
  (1.0, 1.718281828459045),
  (-1.0, -0.6321205588285577),
  (-2.0, -0.8646647167633873),
  (-10.0, -0.9999546000702375),
  (2.0, 6.38905609893065),
  (5.0, 147.4131591025766),
  (10.0, 22025.465794806718),
  (20.0, 485165194.4097903),
  (30.0, 10686474581523.463),
  (-30.0, -0.9999999999999064),
  (1e-17, 1e-17),
  (-1e-17, -1e-17),
  (0.0, 0.0),
  (-0.0, -0.0),
  (5e-324, 5e-324),
  (1e-10, 1.00000000005e-10),
  (0.2, 0.22140275816016985),
  (-0.2, -0.18126924692201815),
  (5.551115123125783e-17, 5.551115123125783e-17),
  (38.816242111356935, 72057594037927780.0),
  (-38.816242111356935, -1.0),
  (709.7, 1.6549840276802644e+308),
];

/// Arguments and the answer of `Math.log` in Node for each of them. They test zeros, negative
/// numbers, subnormal numbers, f near 0 with k zero and not zero, and both forms of the
/// polynomial sum.
const LOG_ROWS: &[(f64, f64)] = &[
  (0.0, f64::NEG_INFINITY),
  (-0.0, f64::NEG_INFINITY),
  (-1.0, f64::NAN),
  (f64::NEG_INFINITY, f64::NAN),
  (f64::NAN, f64::NAN),
  (f64::INFINITY, f64::INFINITY),
  (5e-324, -744.4400719213812),
  (1e-310, -713.8013788281542),
  (1.0, 0.0),
  (2.0, LN_2),
  (1.0000000000000002, 2.2204460492503128e-16),
  (2.0000000000000004, 0.6931471805599455),
  (0.9999999999999999, -1.1102230246251565e-16),
  (1.4, 0.33647223662121284),
  (2.8, 1.0296194171811581),
  (1.2, 0.1823215567939546),
  (10.0, LN_10),
  (0.09, -2.407945608651872),
  (1e+300, 690.7755278982137),
  (0.5, -LN_2),
  (1.3, 0.26236426446749106),
  (0.7, -0.35667494393873245),
  (-2.0, f64::NAN),
];

/// Arguments and the answer of `Math.log1p` in Node for each of them. They test x of -1 and
/// below, |x| near 2^-29 and 2^-54, the k of 0, u near sqrt(2) and near powers of two, and x of
/// 2^53 and more.
const LOG1P_ROWS: &[(f64, f64)] = &[
  (-1.0, f64::NEG_INFINITY),
  (-2.0, f64::NAN),
  (f64::NEG_INFINITY, f64::NAN),
  (1e-20, 1e-20),
  (-0.0, -0.0),
  (0.0, 0.0),
  (1e-10, 9.999999999500001e-11),
  (-1e-10, -1.00000000005e-10),
  (0.2, 0.18232155679395462),
  (-0.2, -0.22314355131420976),
  (0.3, 0.26236426446749106),
  (-0.29, -0.3424903089467759),
  (-0.2928932188134524, -0.3465735902799726),
  (-0.3, -0.35667494393873234),
  (f64::INFINITY, f64::INFINITY),
  (f64::NAN, f64::NAN),
  (0.5, 0.4054651081081644),
  (1.0, LN_2),
  (-0.5, -LN_2),
  (-0.9, -LN_10),
  (-0.999999, -13.815510557935518),
  (100000000000000000000.0, 46.051701859880914),
  (1152921504606847000.0, 41.58883083359672),
  (3.0, 1.3862943611198906),
  (1.0000000000009095, 0.6931471805604),
  (2.9999999, 1.3862943361198903),
  (0.41421356237309503, 0.34657359027997264),
  (0.4142135623730951, 0.3465735902799727),
  (9007199254740992.0, 36.7368005696771),
  (9007199254740990.0, 36.7368005696771),
  (-f64::NAN, f64::NAN),
];

/// Arguments and the answer of `Math.log2` in Node for each of them. They test zeros, negative
/// numbers, subnormal numbers, exact powers of two and x near 1.
const LOG2_ROWS: &[(f64, f64)] = &[
  (0.0, f64::NEG_INFINITY),
  (-0.0, f64::NEG_INFINITY),
  (-1.0, f64::NAN),
  (f64::NAN, f64::NAN),
  (f64::INFINITY, f64::INFINITY),
  (1.0, 0.0),
  (5e-324, -1074.0),
  (1e-310, -1029.7977094150824),
  (2.0, 1.0),
  (8.0, 3.0),
  (0.5, -1.0),
  (3.0, 1.584962500721156),
  (10.0, LOG2_10),
  (1.3, 0.3785116232537299),
  (0.1, -LOG2_10),
  (1e+300, 996.5784284662087),
  (1.0000000000000002, 3.203426503814917e-16),
  (FRAC_1_SQRT_2, -0.4999999999999999),
];

/// Arguments and the answer of `Math.log10` in Node for each of them. They test zeros, negative
/// numbers, subnormal numbers, exact powers of ten and x near 1.
const LOG10_ROWS: &[(f64, f64)] = &[
  (0.0, f64::NEG_INFINITY),
  (-0.0, f64::NEG_INFINITY),
  (-1.0, f64::NAN),
  (f64::NAN, f64::NAN),
  (f64::INFINITY, f64::INFINITY),
  (1.0, 0.0),
  (5e-324, -323.3062153431158),
  (1e-310, -310.0),
  (1e+22, 22.0),
  (0.00001, -5.0),
  (100.0, 2.0),
  (1e+308, 308.0),
  (0.52, -0.2839966563652008),
  (2.0, LOG10_2),
  (0.5, -LOG10_2),
  (7.0, 0.8450980400142568),
  (1e-300, -300.0),
  (1.0000000000000002, 9.64327466553287e-17),
  (0.9999999999999999, -4.8216373327664354e-17),
];

#[test]
fn exp_gives_the_bits_of_node() {
  for &(x, expected) in EXP_ROWS {
    assert_same_bits(exp(x), expected, x);
  }
}

#[test]
fn expm1_gives_the_bits_of_node() {
  for &(x, expected) in EXPM1_ROWS {
    assert_same_bits(expm1(x), expected, x);
  }
}

#[test]
fn log_gives_the_bits_of_node() {
  for &(x, expected) in LOG_ROWS {
    assert_same_bits(log(x), expected, x);
  }
}

#[test]
fn log1p_gives_the_bits_of_node() {
  for &(x, expected) in LOG1P_ROWS {
    assert_same_bits(log1p(x), expected, x);
  }
}

#[test]
fn log2_gives_the_bits_of_node() {
  for &(x, expected) in LOG2_ROWS {
    assert_same_bits(log2(x), expected, x);
  }
}

#[test]
fn log10_gives_the_bits_of_node() {
  for &(x, expected) in LOG10_ROWS {
    assert_same_bits(log10(x), expected, x);
  }
}
