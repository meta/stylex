//! `exp`, `expm1`, `log`, `log1p`, `log2` and `log10` from `ieee754.cc`.

use super::{
  HALF, HUGE, INFINITY_HIGH, LN_MAX_HIGH, ONE, SIGN_MASK, TINY, TWO_M28_HIGH, TWO54, abs_high_word,
  from_words, high_word, low_word, signed_high_word, signed_words, with_high_word, with_low_word,
  without_sign,
};

/// `2^-1000`.
const TWOM1000: f64 = f64::from_bits(0x0170_0000_0000_0000);
/// `2^1023`. The C code of `expm1` writes it as `8.98846567431158e+307`.
const TWO1023: f64 = f64::from_bits(0x7FE0_0000_0000_0000);

/// Above this `x`, `exp(x)` overflows.
const O_THRESHOLD: f64 = f64::from_bits(0x4086_2E42_FEFA_39EF);
/// Below this `x`, `exp(x)` underflows.
const U_THRESHOLD: f64 = f64::from_bits(0xC087_4910_D52D_3051);
/// `1/ln(2)`.
const INVLN2: f64 = f64::from_bits(0x3FF7_1547_652B_82FE);
/// The high part of `ln(2)`. A product with an integer below 2000 is exact.
const LN2_HI: f64 = f64::from_bits(0x3FE6_2E42_FEE0_0000);
/// `ln(2) - LN2_HI`.
const LN2_LO: f64 = f64::from_bits(0x3DEA_39EF_3579_3C76);
/// The value of `Math.E`. `exp(1)` returns it.
const E: f64 = f64::from_bits(0x4005_BF0A_8B14_5769);

// `exp`: R(z) ~ 2.0 + P1*z + P2*z^2 + P3*z^3 + P4*z^4 + P5*z^5.
const P1: f64 = f64::from_bits(0x3FC5_5555_5555_553E);
const P2: f64 = f64::from_bits(0xBF66_C16C_16BE_BD93);
const P3: f64 = f64::from_bits(0x3F11_566A_AF25_DE2C);
const P4: f64 = f64::from_bits(0xBEBB_BD41_C5D2_6BF1);
const P5: f64 = f64::from_bits(0x3E66_3769_72BE_A4D0);

// `expm1`: the Q coefficients, each scaled by 2^n.
const Q1: f64 = f64::from_bits(0xBFA1_1111_1111_10F4);
const Q2: f64 = f64::from_bits(0x3F5A_01A0_19FE_5585);
const Q3: f64 = f64::from_bits(0xBF14_CE19_9EAA_DBB7);
const Q4: f64 = f64::from_bits(0x3ED0_CFCA_86E6_5239);
const Q5: f64 = f64::from_bits(0xBE8A_FDB7_6E09_C32D);

// `log`, `log1p` and `k_log1p`: R(s) ~ Lg1*s^2 + Lg2*s^4 + ... + Lg7*s^14.
const LG1: f64 = f64::from_bits(0x3FE5_5555_5555_5593);
const LG2: f64 = f64::from_bits(0x3FD9_9999_9997_FA04);
const LG3: f64 = f64::from_bits(0x3FD2_4924_9422_9359);
const LG4: f64 = f64::from_bits(0x3FCC_71C5_1D8E_78AF);
const LG5: f64 = f64::from_bits(0x3FC7_4664_96CB_03DE);
const LG6: f64 = f64::from_bits(0x3FC3_9A09_D078_C69F);
const LG7: f64 = f64::from_bits(0x3FC2_F112_DF3E_5244);

/// `0.33333333333333333`, which the C code writes in decimal.
const THIRD: f64 = f64::from_bits(0x3FD5_5555_5555_5555);
/// `0.66666666666666666`, which the C code writes in decimal.
const TWO_THIRDS: f64 = f64::from_bits(0x3FE5_5555_5555_5555);

/// The high part of `1/ln(2)`.
const IVLN2HI: f64 = f64::from_bits(0x3FF7_1547_6520_0000);
/// `1/ln(2) - IVLN2HI`.
const IVLN2LO: f64 = f64::from_bits(0x3DE7_05FC_2EEF_A200);
/// `1/ln(10)`, which the C code writes in decimal.
const IVLN10: f64 = f64::from_bits(0x3FDB_CB7B_1526_E50E);
/// The high 40 bits of `log10(2)`.
const LOG10_2HI: f64 = f64::from_bits(0x3FD3_4413_509F_6000);
/// `log10(2) - LOG10_2HI`.
const LOG10_2LO: f64 = f64::from_bits(0x3D59_FEF3_11F1_2B36);

/// The high word of `ln(2)/2`. Above it, the reduction subtracts `k*ln(2)`.
const HALF_LN2_HIGH: u32 = 0x3FD6_2E42;
/// The high word of `1.5*ln(2)`. Below it, `k` is 1 or -1.
const THREE_HALVES_LN2_HIGH: u32 = 0x3FF0_A2B2;

/// `e` to the power `x`, with an error below 1 unit in the last place.
pub(in super::super) fn exp(x: f64) -> f64 {
  let high = high_word(x);
  let xsb = high & SIGN_MASK;
  let hx = abs_high_word(x);

  // |x| >= 709.78: a NaN, an infinity, an overflow or an underflow.
  if hx >= LN_MAX_HIGH {
    if hx >= INFINITY_HIGH {
      if ((hx & 0xF_FFFF) | low_word(x)) != 0 {
        return x + x;
      }
      return if xsb == 0 { x } else { 0.0 };
    }
    if x > O_THRESHOLD {
      return HUGE * HUGE;
    }
    if x < U_THRESHOLD {
      return TWOM1000 * TWOM1000;
    }
  }

  // Find k and r = hi - lo, so that x = k*ln(2) + r and |r| <= ln(2)/2.
  let (k, hi, lo, x) = if hx > HALF_LN2_HIGH {
    // V8 returns `Math.E` here, because the steps below give a different last
    // bit for `exp(1)`.
    if x == 1.0 {
      return E;
    }
    let (k, hi, lo) = reduce_by_ln2(x, hx, xsb != 0);
    (k, hi, lo, hi - lo)
  } else if hx < TWO_M28_HIGH {
    // |x| < 2^-28. The C code returns this only when `huge + x > one`,
    // which is true for every such `x`. The test only sets the inexact flag.
    return ONE + x;
  } else {
    (0, 0.0, 0.0, x)
  };

  // r is now in the primary range.
  let t = x * x;
  let twopk = if k >= -1021 {
    two_to_k(k)
  } else {
    two_to_k(k + 1000)
  };
  let c = x - t * (P1 + t * (P2 + t * (P3 + t * (P4 + t * P5))));
  if k == 0 {
    return ONE - ((x * c) / (c - 2.0) - x);
  }
  let y = ONE - ((lo - (x * c) / (2.0 - c)) - hi);
  if k >= -1021 {
    return times_two_to_k(y, k, twopk);
  }
  y * twopk * TWOM1000
}

/// Writes `x` as `k*ln(2) + hi - lo`, with `|hi - lo| <= ln(2)/2`, for an `x`
/// with `|x| > ln(2)/2`. `hx` is the high word of `|x|`.
fn reduce_by_ln2(x: f64, hx: u32, negative: bool) -> (i32, f64, f64) {
  if hx < THREE_HALVES_LN2_HIGH {
    if negative {
      (-1, x + LN2_HI, -LN2_LO)
    } else {
      (1, x - LN2_HI, LN2_LO)
    }
  } else {
    // |x| < 746 for each caller, so the truncation fits in an `i32`.
    let k = (INVLN2 * x + if negative { -HALF } else { HALF }) as i32;
    let t = f64::from(k);
    // `t * LN2_HI` is exact.
    (k, x - t * LN2_HI, t * LN2_LO)
  }
}

/// 2^`k`, made from its exponent bits. For a negative `k`, the sum wraps and
/// gives the bits of the signed sum of the C code. For `k` of 1024, the result
/// is an infinity, and [`times_two_to_k`] does not read it.
fn two_to_k(k: i32) -> f64 {
  from_words(0x3FF0_0000_u32.wrapping_add((k as u32) << 20), 0)
}

/// `y * 2^k`, where `twopk` is 2^k for every `k` except 1024. 2^1024 is not a
/// double, so for that `k` the last step is a separate product.
fn times_two_to_k(y: f64, k: i32, twopk: f64) -> f64 {
  if k == 1024 {
    y * 2.0 * TWO1023
  } else {
    y * twopk
  }
}

/// `exp(x) - 1`, with an error below 1 unit in the last place, also when `x`
/// is near 0.
pub(in super::super) fn expm1(x: f64) -> f64 {
  let high = high_word(x);
  let xsb = high & SIGN_MASK;
  let hx = abs_high_word(x);

  // |x| >= 56*ln(2): a NaN, an infinity, an overflow, or -1.
  if hx >= 0x4043_687A {
    if hx >= LN_MAX_HIGH {
      if hx >= INFINITY_HIGH {
        if ((hx & 0xF_FFFF) | low_word(x)) != 0 {
          return x + x;
        }
        return if xsb == 0 { x } else { -1.0 };
      }
      if x > O_THRESHOLD {
        return HUGE * HUGE;
      }
    }
    if xsb != 0 {
      // x < -56*ln(2), so `exp(x)` is less than half a unit of -1. The C code
      // returns this only when `x + tiny < 0`, which is true for every such
      // finite `x`. The test only sets the inexact flag.
      return TINY - ONE;
    }
  }

  // Find k and r = hi - lo, so that x = k*ln(2) + r and |r| <= ln(2)/2. The
  // rounding error of r is c.
  let (k, x, c) = if hx > HALF_LN2_HIGH {
    let (k, hi, lo) = reduce_by_ln2(x, hx, xsb != 0);
    let r = hi - lo;
    (k, r, (hi - r) - lo)
  } else if hx < 0x3C90_0000 {
    // |x| < 2^-54. The C code returns `x - (t - (huge + x))` with
    // `t = huge + x`. The two sums are equal, so the difference is +0, and
    // `x - 0` is `x` also for -0. The sums only set the inexact flag.
    return x;
  } else {
    (0, x, 0.0)
  };

  // r is now in the primary range.
  let hfx = 0.5 * x;
  let hxs = x * hfx;
  let r1 = ONE + hxs * (Q1 + hxs * (Q2 + hxs * (Q3 + hxs * (Q4 + hxs * Q5))));
  let t = 3.0 - r1 * hfx;
  let e = hxs * ((r1 - t) / (6.0 - x * t));
  if k == 0 {
    // c is 0.
    return x - (x * e - hxs);
  }
  let twopk = two_to_k(k);
  let e = (x * (e - c) - c) - hxs;
  if k == -1 {
    return 0.5 * (x - e) - 0.5;
  }
  if k == 1 {
    if x < -0.25 {
      return -2.0 * (e - (x + 0.5));
    }
    return ONE + 2.0 * (x - e);
  }
  if k <= -2 || k > 56 {
    let y = ONE - (e - x);
    return times_two_to_k(y, k, twopk) - ONE;
  }
  // 2 <= k <= 56 here, so each shift below is in range.
  if k < 20 {
    // 1 - 2^-k.
    let t = with_high_word(ONE, 0x3FF0_0000 - (0x0020_0000 >> k));
    (t - (e - x)) * twopk
  } else {
    // 2^-k.
    let t = with_high_word(ONE, ((0x3FF - k) as u32) << 20);
    let y = x - (e + t);
    (y + ONE) * twopk
  }
}

/// The natural logarithm of `x`, with an error below 1 unit in the last place.
pub(in super::super) fn log(x: f64) -> f64 {
  let (x, k) = match normal_log_argument(x) {
    Ok(argument) => argument,
    Err(answer) => return answer,
  };
  let (k, f, hx) = split_log_argument(x, k);

  // -2^-20 <= f < 2^-20.
  if (0x000F_FFFF & (2 + hx)) < 3 {
    if f == 0.0 {
      if k == 0 {
        return 0.0;
      }
      let dk = f64::from(k);
      return dk * LN2_HI + dk * LN2_LO;
    }
    let r = f * f * (0.5 - THIRD * f);
    if k == 0 {
      return f - r;
    }
    let dk = f64::from(k);
    return dk * LN2_HI - ((r - dk * LN2_LO) - f);
  }

  let s = f / (2.0 + f);
  let dk = f64::from(k);
  let i = hx - 0x6_147A;
  let j = 0x6_B851 - hx;
  let r = log_series(s * s);
  // `i | j` is positive when f is far from 0, and then the longer form is
  // more accurate.
  if (i | j) > 0 {
    let hfsq = 0.5 * f * f;
    if k == 0 {
      return f - (hfsq - s * (hfsq + r));
    }
    return dk * LN2_HI - ((hfsq - (s * (hfsq + r) + dk * LN2_LO)) - f);
  }
  if k == 0 {
    return f - s * (f - r);
  }
  dk * LN2_HI - ((s * (f - r) - dk * LN2_LO) - f)
}

/// `ln(1 + x)`, with an error below 1 unit in the last place, also when `x`
/// is near 0.
pub(in super::super) fn log1p(x: f64) -> f64 {
  let hx = signed_high_word(x);
  let ax = without_sign(hx);

  // 1 + x < sqrt(2).
  if hx < 0x3FDA_827A {
    // x <= -1.
    if ax >= 0x3FF0_0000 {
      if x == -1.0 {
        return f64::NEG_INFINITY;
      }
      return f64::NAN;
    }
    // |x| < 2^-29.
    if ax < 0x3E20_0000 {
      // The C code also tests `two54 + x > zero`, which is true for every
      // such `x`. The test only sets the inexact flag.
      if ax < 0x3C90_0000 {
        return x;
      }
      return x - x * x * 0.5;
    }
    // sqrt(2)/2 <= 1 + x < sqrt(2): k = 0 and f = x.
    if hx > 0 || hx <= 0xBFD2_BEC4_u32 as i32 {
      let f = x;
      let hfsq = 0.5 * f * f;
      let s = f / (2.0 + f);
      return f - (hfsq - s * (hfsq + log1p_poly(s * s)));
    }
  }
  if hx >= INFINITY_HIGH as i32 {
    return x + x;
  }

  // Here 1 + x is outside [sqrt(2)/2, sqrt(2)), so k below is never 0.
  // Write 1 + x as 2^k * u. The rounding error of 1 + x, divided by u, is c.
  let (mut k, mut u, c) = if hx < 0x4340_0000 {
    let u = 1.0 + x;
    let k = (high_word(u) as i32 >> 20) - 1023;
    let c = if k > 0 { 1.0 - (u - x) } else { x - (u - 1.0) };
    (k, u, c / u)
  } else {
    // x >= 2^53, so 1 + x rounds to x.
    (((hx >> 20) - 1023), x, 0.0)
  };
  let mut hu = high_word(u) & 0x000F_FFFF;
  if hu < 0x6_A09E {
    // Make u a number in [1, sqrt(2)).
    u = with_high_word(u, hu | 0x3FF0_0000);
  } else {
    // Make u/2 a number in [sqrt(2)/2, 1).
    k += 1;
    u = with_high_word(u, hu | 0x3FE0_0000);
    hu = (0x0010_0000 - hu) >> 2;
  }
  let f = u - 1.0;
  let dk = f64::from(k);
  let hfsq = 0.5 * f * f;

  // |f| < 2^-20.
  if hu == 0 {
    if f == 0.0 {
      return dk * LN2_HI + (c + dk * LN2_LO);
    }
    let r = hfsq * (1.0 - TWO_THIRDS * f);
    return dk * LN2_HI - ((r - (dk * LN2_LO + c)) - f);
  }
  let s = f / (2.0 + f);
  let r = log1p_poly(s * s);
  dk * LN2_HI - ((hfsq - (s * (hfsq + r) + (dk * LN2_LO + c))) - f)
}

/// The answer of a logarithm for a zero, a negative number, an infinity or a
/// NaN. For any other `x`: `x`, with a subnormal number scaled by 2^54, and the
/// power of two that the scale adds.
fn normal_log_argument(x: f64) -> Result<(f64, i32), f64> {
  let (hx, low) = signed_words(x);

  // x < 2^-1022: a zero, a negative number or a subnormal number.
  if hx < 0x0010_0000 {
    if (abs_high_word(x) | low) == 0 {
      return Err(f64::NEG_INFINITY);
    }
    if hx < 0 {
      return Err(f64::NAN);
    }
    return Ok((x * TWO54, -54));
  }
  if hx >= INFINITY_HIGH as i32 {
    return Err(x + x);
  }
  Ok((x, 0))
}

/// Writes the normal `x` times 2^`k` as 2^k * (1 + f), with 1 + f in
/// [sqrt(2)/2, sqrt(2)). Gives the new `k`, `f`, and the fraction bits of the
/// high word of `x`.
fn split_log_argument(x: f64, k: i32) -> (i32, f64, i32) {
  let hx = signed_high_word(x);
  let k = k + (hx >> 20) - 1023;
  let hx = hx & 0x000F_FFFF;
  let i = (hx + 0x9_5F64) & 0x10_0000;
  // Make x or x/2 a number in [1, 2).
  let x = with_high_word(x, (hx | (i ^ 0x3FF0_0000)) as u32);
  (k + (i >> 20), x - 1.0, hx)
}

/// The series R(z) of `log` and `k_log1p`, with `z = s^2`, as two sums of
/// alternate terms.
fn log_series(z: f64) -> f64 {
  let w = z * z;
  let t1 = w * (LG2 + w * (LG4 + w * LG6));
  let t2 = z * (LG1 + w * (LG3 + w * (LG5 + w * LG7)));
  t2 + t1
}

/// The polynomial R(z) of `log1p`, with `z = s^2`. It is the series of
/// [`log_series`] in one sum, which rounds differently, so the two stay
/// separate.
fn log1p_poly(z: f64) -> f64 {
  z * (LG1 + z * (LG2 + z * (LG3 + z * (LG4 + z * (LG5 + z * (LG6 + z * LG7))))))
}

/// `ln(1 + f) - f` for `1 + f` in about `[sqrt(2)/2, sqrt(2)]`.
fn k_log1p(f: f64) -> f64 {
  let s = f / (2.0 + f);
  let hfsq = 0.5 * f * f;
  s * (hfsq + log_series(s * s))
}

/// The base 2 logarithm of `x`.
pub(in super::super) fn log2(x: f64) -> f64 {
  let (x, k) = match normal_log_argument(x) {
    Ok(argument) => argument,
    Err(answer) => return answer,
  };
  // The C code reads the low word before the scale of a subnormal number. A
  // scaled subnormal number is far from 1, so the test is the same.
  if x == 1.0 {
    return 0.0;
  }
  let (k, f, _) = split_log_argument(x, k);
  let y = f64::from(k);
  let hfsq = 0.5 * f * f;
  let r = k_log1p(f);

  // Split f - hfsq into hi + lo, with the low word of hi zero, so that the
  // products with IVLN2HI below are exact.
  let hi = with_low_word(f - hfsq, 0);
  let lo = (f - hi) - hfsq + r;
  let val_hi = hi * IVLN2HI;
  let val_lo = (lo + hi) * IVLN2LO + lo * IVLN2HI;

  // Add y to val_hi and keep the rounding error in val_lo.
  let w = y + val_hi;
  let val_lo = val_lo + ((y - w) + val_hi);
  let val_hi = w;

  val_lo + val_hi
}

/// The base 10 logarithm of `x`.
pub(in super::super) fn log10(x: f64) -> f64 {
  let (x, k) = match normal_log_argument(x) {
    Ok(argument) => argument,
    Err(answer) => return answer,
  };
  if x == 1.0 {
    return 0.0;
  }
  let (mut hx, lx) = signed_words(x);
  let k = k + (hx >> 20) - 1023;

  // i is 1 when k is negative. Then the code scales x to [0.5, 1), not to
  // [1, 2).
  let i = i32::from(k < 0);
  hx = (hx & 0x000F_FFFF) | ((0x3FF - i) << 20);
  let y = f64::from(k + i);
  let x = from_words(hx as u32, lx);

  let z = y * LOG10_2LO + IVLN10 * log(x);
  z + y * LOG10_2HI
}

#[cfg(test)]
#[path = "tests/exp_log_tests.rs"]
mod exp_log_tests;
