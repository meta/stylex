//! `sinh`, `cosh`, `tanh`, `asinh`, `acosh` and `atanh` from `ieee754.cc`.
//!
//! Each one calls the ports of `exp`, `expm1`, `log` and `log1p`, as the C
//! code calls the fdlibm functions of V8.

use super::exp_log::{exp, expm1, log, log1p};
use super::{
  HALF, HUGE, INFINITY_HIGH, LN_MAX_HIGH, ONE, TINY, TWO_M28_HIGH, abs_high_word,
  high_with_low_flag, signed_high_word, signed_words, with_high_word, without_sign,
};

/// `ln(2)`.
const LN2: f64 = f64::from_bits(0x3FE6_2E42_FEFA_39EF);
/// `2^-28`, which the C code of `sinh` writes in decimal.
const TWO_M28: f64 = f64::from_bits(0x3E30_0000_0000_0000);
/// `709.7822265625`, the high word of `ln(MAX_VALUE)` with a zero low word.
const LOG_MAXD: f64 = f64::from_bits(0x4086_2E42_0000_0000);
/// `710.4758600739439`. Above it, `sinh` and `cosh` overflow.
const OVERFLOW_THRESHOLD: f64 = f64::from_bits(0x4086_33CE_8FB9_F87D);
/// `1.0e307`, which the C code of `sinh` writes in decimal.
const SHUGE: f64 = f64::from_bits(0x7FAC_7B1F_3CAC_7433);

/// The high word of `22`.
const TWENTY_TWO_HIGH: u32 = 0x4036_0000;

/// The hyperbolic sine of `x`.
pub(in super::super) fn sinh(x: f64) -> f64 {
  let h = if x < 0.0 { -HALF } else { HALF };
  let ax = x.abs();

  // |x| < 22: sign(x) * 0.5 * (E + E/(E + 1)), with E = expm1(|x|).
  if ax < 22.0 {
    // sinh(x) is x for |x| < 2^-28.
    if ax < TWO_M28 {
      return x;
    }
    let t = expm1(ax);
    if ax < 1.0 {
      return h * (2.0 * t - t * t / (t + ONE));
    }
    return h * (t + t / (t + ONE));
  }

  // |x| in [22, ln(MAX_VALUE)]: sign(x) * 0.5 * exp(|x|).
  if ax < LOG_MAXD {
    return h * exp(ax);
  }

  // |x| up to the overflow threshold: two halves, so that no step overflows.
  if ax <= OVERFLOW_THRESHOLD {
    let w = exp(HALF * ax);
    let t = h * w;
    return t * w;
  }

  // An overflow gives an infinity with the sign of x. A NaN stays NaN.
  x * SHUGE
}

/// The hyperbolic cosine of `x`.
pub(in super::super) fn cosh(x: f64) -> f64 {
  let ix = abs_high_word(x);

  // |x| in [0, ln(2)/2]: 1 + expm1(|x|)^2 / (2 * exp(|x|)).
  if ix < 0x3FD6_2E43 {
    let t = expm1(x.abs());
    let w = ONE + t;
    // cosh(x) is 1 for |x| < 2^-55.
    if ix < 0x3C80_0000 {
      return w;
    }
    return ONE + (t * t) / (w + w);
  }

  // |x| in [ln(2)/2, 22]: (exp(|x|) + 1/exp(|x|)) / 2.
  if ix < TWENTY_TWO_HIGH {
    let t = exp(x.abs());
    return HALF * t + HALF / t;
  }

  // |x| in [22, ln(MAX_VALUE)]: exp(|x|) / 2.
  if ix < LN_MAX_HIGH {
    return HALF * exp(x.abs());
  }

  // |x| up to the overflow threshold: two halves, so that no step overflows.
  if x.abs() <= OVERFLOW_THRESHOLD {
    let w = exp(HALF * x.abs());
    let t = HALF * w;
    return t * w;
  }

  // x is an infinity or NaN.
  if ix >= INFINITY_HIGH {
    return x * x;
  }

  // The overflow gives +Infinity.
  HUGE * HUGE
}

/// The hyperbolic tangent of `x`.
pub(in super::super) fn tanh(x: f64) -> f64 {
  let jx = signed_high_word(x);
  let ix = without_sign(jx);

  // tanh(+-Infinity) is +-1, and tanh(NaN) is NaN.
  if ix >= INFINITY_HIGH as i32 {
    if jx >= 0 {
      return ONE / x + ONE;
    }
    return ONE / x - ONE;
  }

  let z = if ix < TWENTY_TWO_HIGH as i32 {
    // |x| < 2^-28. The C code returns x only when `huge + x > one`, which is
    // true for every such x. The test only sets the inexact flag.
    if ix < TWO_M28_HIGH as i32 {
      return x;
    }
    if ix >= 0x3FF0_0000 {
      // |x| >= 1: 1 - 2/(expm1(2|x|) + 2).
      let t = expm1(2.0 * x.abs());
      ONE - 2.0 / (t + 2.0)
    } else {
      // |x| < 1: -expm1(-2|x|) / (expm1(-2|x|) + 2).
      let t = expm1(-2.0 * x.abs());
      -t / (t + 2.0)
    }
  } else {
    // |x| >= 22: the difference rounds to 1.
    ONE - TINY
  };
  if jx >= 0 { z } else { -z }
}

/// The inverse hyperbolic sine of `x`.
pub(in super::super) fn asinh(x: f64) -> f64 {
  let hx = signed_high_word(x);
  let ix = without_sign(hx);

  // x is an infinity or NaN.
  if ix >= INFINITY_HIGH as i32 {
    return x + x;
  }
  // |x| < 2^-28. The C code returns x only when `huge + x > one`, which is
  // true for every such x. The test only sets the inexact flag.
  if ix < TWO_M28_HIGH as i32 {
    return x;
  }

  let w = if ix > 0x41B0_0000 {
    // |x| > 2^28: ln(|x|) + ln(2).
    log(x.abs()) + LN2
  } else if ix > 0x4000_0000 {
    // 2 < |x| <= 2^28: ln(2|x| + 1/(sqrt(x^2 + 1) + |x|)).
    let t = x.abs();
    log(2.0 * t + ONE / ((x * x + ONE).sqrt() + t))
  } else {
    // 2^-28 <= |x| <= 2: log1p(|x| + x^2/(1 + sqrt(1 + x^2))).
    let t = x * x;
    log1p(x.abs() + t / (ONE + (ONE + t).sqrt()))
  };
  if hx > 0 { w } else { -w }
}

/// The inverse hyperbolic cosine of `x`.
pub(in super::super) fn acosh(x: f64) -> f64 {
  let (hx, lx) = signed_words(x);

  // x < 1, or a NaN with the sign bit.
  if hx < 0x3FF0_0000 {
    return f64::NAN;
  }
  if hx >= 0x41B0_0000 {
    // x is +Infinity or NaN.
    if hx >= INFINITY_HIGH as i32 {
      return x + x;
    }
    // x >= 2^28: ln(x) + ln(2).
    return log(x) + LN2;
  }
  // acosh(1) is +0.
  if ((hx - 0x3FF0_0000) as u32 | lx) == 0 {
    return 0.0;
  }
  if hx > 0x4000_0000 {
    // 2 < x < 2^28: ln(2x - 1/(x + sqrt(x^2 - 1))).
    let t = x * x;
    return log(2.0 * x - ONE / (x + (t - ONE).sqrt()));
  }
  // 1 < x <= 2: log1p(t + sqrt(2t + t^2)), with t = x - 1.
  let t = x - ONE;
  log1p(t + (2.0 * t + t * t).sqrt())
}

/// The inverse hyperbolic tangent of `x`.
pub(in super::super) fn atanh(x: f64) -> f64 {
  let (hx, lx) = signed_words(x);
  let ix = without_sign(hx);

  // |x| > 1 or NaN. A low word that is not zero sets bit 0 of ix, so |x| just
  // above 1 is also caught.
  if high_with_low_flag(ix, lx) > 0x3FF0_0000 {
    return f64::NAN;
  }
  // atanh(+-1) is +-Infinity.
  if ix == 0x3FF0_0000 {
    return if x > 0.0 {
      f64::INFINITY
    } else {
      f64::NEG_INFINITY
    };
  }
  // |x| < 2^-28. The C code returns x only when `huge + x > zero`, which is
  // true for every such x. The test only sets the inexact flag.
  if ix < TWO_M28_HIGH as i32 {
    return x;
  }

  // From here, x is |x|.
  let x = with_high_word(x, ix as u32);
  let t = if ix < 0x3FE0_0000 {
    // |x| < 0.5: 0.5 * log1p(2x + 2x*x/(1 - x)).
    let t = x + x;
    0.5 * log1p(t + t * x / (ONE - x))
  } else {
    // |x| >= 0.5: 0.5 * log1p(2x/(1 - x)).
    0.5 * log1p((x + x) / (ONE - x))
  };
  if hx >= 0 { t } else { -t }
}

#[cfg(test)]
#[path = "tests/hyperbolic_tests.rs"]
mod hyperbolic_tests;
