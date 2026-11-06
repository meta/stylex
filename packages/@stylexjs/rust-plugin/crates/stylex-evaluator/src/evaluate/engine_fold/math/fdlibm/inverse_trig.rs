//! `asin`, `acos`, `atan` and `atan2` from `ieee754.cc`.

use super::{
  INFINITY_HIGH, PIO4, TINY, TWO_M27_HIGH, from_words, high_with_low_flag, high_word, signed_words,
  without_sign,
};

const PI: f64 = f64::from_bits(0x4009_21FB_5444_2D18);
const PIO2_HI: f64 = f64::from_bits(0x3FF9_21FB_5444_2D18);
const PIO2_LO: f64 = f64::from_bits(0x3C91_A626_3314_5C07);

// The coefficients of R(x^2), a rational approximation of (asin(x) - x) / x^3.
const PS0: f64 = f64::from_bits(0x3FC5_5555_5555_5555);
const PS1: f64 = f64::from_bits(0xBFD4_D612_03EB_6F7D);
const PS2: f64 = f64::from_bits(0x3FC9_C155_0E88_4455);
const PS3: f64 = f64::from_bits(0xBFA4_8228_B568_8F3B);
const PS4: f64 = f64::from_bits(0x3F49_EFE0_7501_B288);
const PS5: f64 = f64::from_bits(0x3F02_3DE1_0DFD_F709);
const QS1: f64 = f64::from_bits(0xC003_3A27_1C8A_2D4B);
const QS2: f64 = f64::from_bits(0x4000_2AE5_9C59_8AC8);
const QS3: f64 = f64::from_bits(0xBFE6_066C_1B8D_0159);
const QS4: f64 = f64::from_bits(0x3FB3_B8C5_B12E_9282);

/// The high and the low part of atan(0.5), atan(1), atan(1.5) and atan(inf).
const ATAN_HI_LO: [(f64, f64); 4] = [
  (
    f64::from_bits(0x3FDD_AC67_0561_BB4F),
    f64::from_bits(0x3C7A_2B7F_222F_65E2),
  ),
  (
    f64::from_bits(0x3FE9_21FB_5444_2D18),
    f64::from_bits(0x3C81_A626_3314_5C07),
  ),
  (
    f64::from_bits(0x3FEF_730B_D281_F69B),
    f64::from_bits(0x3C70_0788_7AF0_CBBD),
  ),
  (
    f64::from_bits(0x3FF9_21FB_5444_2D18),
    f64::from_bits(0x3C91_A626_3314_5C07),
  ),
];

/// The coefficients of the atan polynomial.
const AT: [f64; 11] = [
  f64::from_bits(0x3FD5_5555_5555_550D),
  f64::from_bits(0xBFC9_9999_9998_EBC4),
  f64::from_bits(0x3FC2_4924_9200_83FF),
  f64::from_bits(0xBFBC_71C6_FE23_1671),
  f64::from_bits(0x3FB7_45CD_C54C_206E),
  f64::from_bits(0xBFB3_B0F2_AF74_9A6D),
  f64::from_bits(0x3FB1_0D66_A0D0_3D51),
  f64::from_bits(0xBFAD_DE2D_52DE_FD9A),
  f64::from_bits(0x3FA9_7B4B_2476_0DEB),
  f64::from_bits(0xBFA2_B444_2C6A_6C2F),
  f64::from_bits(0x3F90_AD3A_E322_DA11),
];

/// The numerator and the denominator of R(`z`).
fn asin_ratio_terms(z: f64) -> (f64, f64) {
  let p = z * (PS0 + z * (PS1 + z * (PS2 + z * (PS3 + z * (PS4 + z * PS5)))));
  let q = 1.0 + z * (QS1 + z * (QS2 + z * (QS3 + z * QS4)));
  (p, q)
}

/// The arcsine of `x`, in radians.
pub(in super::super) fn asin(x: f64) -> f64 {
  let (hx, lx) = signed_words(x);
  let ix = without_sign(hx);

  if ix >= 0x3FF0_0000 {
    // asin(+-1) is +-pi/2. NaN and |x| > 1 give NaN.
    if ((ix - 0x3FF0_0000) as u32 | lx) == 0 {
      return x * PIO2_HI + x * PIO2_LO;
    }
    return f64::NAN;
  } else if ix < 0x3FE0_0000 {
    // For |x| < 2^-27 the C code tests `huge + x > one` only to raise the
    // inexact flag. The test is always true there, so the result is `x`.
    if ix < TWO_M27_HIGH as i32 {
      return x;
    }
    let t = x * x;
    let (p, q) = asin_ratio_terms(t);
    let w = p / q;
    return x + x * w;
  }

  // 0.5 <= |x| < 1: asin(x) = pi/2 - 2 * asin(sqrt((1 - |x|) / 2)).
  let w = 1.0 - x.abs();
  let t = w * 0.5;
  let (p, q) = asin_ratio_terms(t);
  let s = t.sqrt();
  let t = if ix >= 0x3FEF_3333 {
    // |x| > 0.975.
    let w = p / q;
    PIO2_HI - (2.0 * (s + s * w) - PIO2_LO)
  } else {
    // Split sqrt(t) into a high part `w` and a correction `c`.
    let w = from_words(high_word(s), 0);
    let c = (t - w * w) / (s + w);
    let r = p / q;
    let p = 2.0 * s * r - (PIO2_LO - 2.0 * c);
    let q = PIO4 - 2.0 * w;
    PIO4 - (p - q)
  };
  if hx > 0 { t } else { -t }
}

/// The arccosine of `x`, in radians.
pub(in super::super) fn acos(x: f64) -> f64 {
  let (hx, lx) = signed_words(x);
  let ix = without_sign(hx);

  if ix >= 0x3FF0_0000 {
    // acos(1) is 0 and acos(-1) is pi. NaN and |x| > 1 give NaN.
    if ((ix - 0x3FF0_0000) as u32 | lx) == 0 {
      if hx > 0 {
        return 0.0;
      }
      return PI + 2.0 * PIO2_LO;
    }
    return f64::NAN;
  }

  if ix < 0x3FE0_0000 {
    // |x| < 0.5: acos(x) = pi/2 - asin(x).
    if ix <= 0x3C60_0000 {
      return PIO2_HI + PIO2_LO;
    }
    let z = x * x;
    let (p, q) = asin_ratio_terms(z);
    let r = p / q;
    PIO2_HI - (x - (PIO2_LO - x * r))
  } else if hx < 0 {
    // x < -0.5: acos(x) = pi - 2 * asin(sqrt((1 + x) / 2)).
    let z = (1.0 + x) * 0.5;
    let (p, q) = asin_ratio_terms(z);
    let s = z.sqrt();
    let r = p / q;
    let w = r * s - PIO2_LO;
    PI - 2.0 * (s + w)
  } else {
    // x > 0.5: acos(x) = 2 * asin(sqrt((1 - x) / 2)), with sqrt split into a
    // high part `df` and a correction `c`.
    let z = (1.0 - x) * 0.5;
    let s = z.sqrt();
    let df = from_words(high_word(s), 0);
    let c = (z - df * df) / (s + df);
    let (p, q) = asin_ratio_terms(z);
    let r = p / q;
    let w = r * s + c;
    2.0 * (df + w)
  }
}

/// The arctangent of `x`, in radians.
pub(in super::super) fn atan(x: f64) -> f64 {
  let (hx, lx) = signed_words(x);
  let ix = without_sign(hx);

  if ix >= 0x4410_0000 {
    // |x| >= 2^66, an infinity or NaN.
    if ix > INFINITY_HIGH as i32 || (ix == INFINITY_HIGH as i32 && lx != 0) {
      return x + x;
    }
    let (hi, lo) = ATAN_HI_LO[3];
    if hx > 0 {
      return hi + lo;
    }
    return -hi - lo;
  }

  // Reduce |x| to a small `x` and the atan of a point near |x|. The indexes
  // into ATAN_HI_LO are constants, so each one is in range.
  let (x, near) = if ix < 0x3FDC_0000 {
    // |x| < 0.4375. For |x| < 2^-27 the C code tests `huge + x > one` only to
    // raise the inexact flag. The test is always true there, so the result is
    // `x`.
    if ix < TWO_M27_HIGH as i32 {
      return x;
    }
    (x, None)
  } else {
    let x = x.abs();
    if ix < 0x3FF3_0000 {
      if ix < 0x3FE6_0000 {
        // 7/16 <= |x| < 11/16.
        ((2.0 * x - 1.0) / (2.0 + x), Some(ATAN_HI_LO[0]))
      } else {
        // 11/16 <= |x| < 19/16.
        ((x - 1.0) / (x + 1.0), Some(ATAN_HI_LO[1]))
      }
    } else if ix < 0x4003_8000 {
      // 19/16 <= |x| < 2.4375.
      ((x - 1.5) / (1.0 + 1.5 * x), Some(ATAN_HI_LO[2]))
    } else {
      // 2.4375 <= |x| < 2^66.
      (-1.0 / x, Some(ATAN_HI_LO[3]))
    }
  };

  // The sum of AT[i] * z^(i + 1), split into the odd and the even terms.
  let z = x * x;
  let w = z * z;
  let s1 = z * (AT[0] + w * (AT[2] + w * (AT[4] + w * (AT[6] + w * (AT[8] + w * AT[10])))));
  let s2 = w * (AT[1] + w * (AT[3] + w * (AT[5] + w * (AT[7] + w * AT[9]))));
  match near {
    None => x - x * (s1 + s2),
    Some((hi, lo)) => {
      let z = hi - ((x * (s1 + s2) - lo) - x);
      if hx < 0 { -z } else { z }
    },
  }
}

/// The angle of the point (`x`, `y`) from the positive x axis, in radians.
pub(in super::super) fn atan2(y: f64, x: f64) -> f64 {
  // The C code adds `tiny` to some results only to raise the inexact flag.
  // The sum is the same number, so the port keeps it as in the C code.
  const PI_LO: f64 = f64::from_bits(0x3CA1_A626_3314_5C07);

  let (hx, lx) = signed_words(x);
  let ix = without_sign(hx);
  let (hy, ly) = signed_words(y);
  let iy = without_sign(hy);

  // x or y is NaN.
  if high_with_low_flag(ix, lx) > INFINITY_HIGH || high_with_low_flag(iy, ly) > INFINITY_HIGH {
    return x + y;
  }
  // `hx` is negative for a negative x, and i32::MIN - 0x3FF00000 overflows.
  // So the subtraction wraps, as the C code does.
  if (hx.wrapping_sub(0x3FF0_0000) as u32 | lx) == 0 {
    return atan(y);
  }
  // 2 * sign(x) + sign(y). The shifts are arithmetic, as for an int32 in C.
  let m = ((hy >> 31) & 1) | ((hx >> 30) & 2);

  // y is zero.
  if (iy as u32 | ly) == 0 {
    return match m {
      0 | 1 => y,
      2 => PI + TINY,
      _ => -PI - TINY,
    };
  }
  // x is zero.
  if (ix as u32 | lx) == 0 {
    return if hy < 0 {
      -PIO2_HI - TINY
    } else {
      PIO2_HI + TINY
    };
  }
  // x is an infinity.
  if ix == INFINITY_HIGH as i32 {
    if iy == INFINITY_HIGH as i32 {
      return match m {
        0 => PIO4 + TINY,
        1 => -PIO4 - TINY,
        2 => 3.0 * PIO4 + TINY,
        _ => -3.0 * PIO4 - TINY,
      };
    }
    return match m {
      0 => 0.0,
      1 => -0.0,
      2 => PI + TINY,
      _ => -PI - TINY,
    };
  }
  // y is an infinity.
  if iy == INFINITY_HIGH as i32 {
    return if hy < 0 {
      -PIO2_HI - TINY
    } else {
      PIO2_HI + TINY
    };
  }

  // The difference of the exponents of y and x.
  let k = (iy - ix) >> 20;
  let (z, m) = if k > 60 {
    // |y / x| > 2^60.
    (PIO2_HI + 0.5 * PI_LO, m & 1)
  } else if hx < 0 && k < -60 {
    // 0 > |y| / x > -2^-60.
    (0.0, m)
  } else {
    (atan((y / x).abs()), m)
  };
  match m {
    0 => z,
    1 => -z,
    2 => PI - (z - PI_LO),
    _ => (z - PI_LO) - PI,
  }
}

#[cfg(test)]
#[path = "tests/inverse_trig_tests.rs"]
mod inverse_trig_tests;
