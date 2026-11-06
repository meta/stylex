//! `sin`, `cos`, `tan` and their kernels from `ieee754.cc`.
//!
//! Each function first reduces `x` to `n * pi/2 + y` with `|y| <= pi/4`. Let
//! S, C and T be the kernels on `y`. Then:
//!
//! | n mod 4 | sin | cos | tan  |
//! | ------- | --- | --- | ---- |
//! | 0       | S   | C   | T    |
//! | 1       | C   | -S  | -1/T |
//! | 2       | -S  | -C  | T    |
//! | 3       | -C  | S   | -1/T |
//!
//! The sine, the cosine and the tangent of an infinity or of NaN is NaN.

use super::{
  HALF, INFINITY_HIGH, ONE, PIO4, TWO_M27_HIGH, TWO_M28_HIGH, abs_high_word, from_words,
  rem_pio2::rem_pio2, signed_high_word, with_low_word, without_sign,
};

// sin(x) / x - (1 + S1*x^2 + ... + S6*x^12) < 2^-58 on [0, pi/4].
const S1: f64 = f64::from_bits(0xBFC5_5555_5555_5549);
const S2: f64 = f64::from_bits(0x3F81_1111_1110_F8A6);
const S3: f64 = f64::from_bits(0xBF2A_01A0_19C1_61D5);
const S4: f64 = f64::from_bits(0x3EC7_1DE3_57B1_FE7D);
const S5: f64 = f64::from_bits(0xBE5A_E5E6_8A2B_9CEB);
const S6: f64 = f64::from_bits(0x3DE5_D93A_5ACF_D57C);

// cos(x) - (1 - x^2/2 + C1*x^4 + ... + C6*x^14) < 2^-58 on [0, pi/4].
const C1: f64 = f64::from_bits(0x3FA5_5555_5555_554C);
const C2: f64 = f64::from_bits(0xBF56_C16C_16C1_5177);
const C3: f64 = f64::from_bits(0x3EFA_01A0_19CB_1590);
const C4: f64 = f64::from_bits(0xBE92_7E4F_809C_52AD);
const C5: f64 = f64::from_bits(0x3E21_EE9E_BDB4_B1C4);
const C6: f64 = f64::from_bits(0xBDA8_FAE9_BE88_38D4);

// tan(x) / x - (1 + T[0]*x^2 + ... + T[12]*x^26) < 2^-59.2 on [0, 0.67434].
const T: [f64; 13] = [
  f64::from_bits(0x3FD5_5555_5555_5563),
  f64::from_bits(0x3FC1_1111_1110_FE7A),
  f64::from_bits(0x3FAB_A1BA_1BB3_41FE),
  f64::from_bits(0x3F96_64F4_8406_D637),
  f64::from_bits(0x3F82_26E3_E96E_8493),
  f64::from_bits(0x3F6D_6D22_C956_0328),
  f64::from_bits(0x3F57_DBC8_FEE0_8315),
  f64::from_bits(0x3F43_44D8_F2F2_6501),
  f64::from_bits(0x3F30_26F7_1A8D_1068),
  f64::from_bits(0x3F14_7E88_A037_92A6),
  f64::from_bits(0x3F12_B80F_32F0_A7E9),
  f64::from_bits(0xBEF3_75CB_DB60_5373),
  f64::from_bits(0x3EFB_2A70_74BF_7AD4),
];
/// The part of `pi/4` that `PIO4` does not hold.
const PIO4LO: f64 = f64::from_bits(0x3C81_A626_3314_5C07);

/// The high word of `pi/4`. A smaller `|x|` needs no reduction.
const PIO4_HIGH: u32 = 0x3FE9_21FB;

/// The sine of `x`.
pub(in super::super) fn sin(x: f64) -> f64 {
  let ix = abs_high_word(x);
  if ix <= PIO4_HIGH {
    kernel_sin(x, 0.0, 0)
  } else if ix >= INFINITY_HIGH {
    // The C code returns `x - x`. JavaScript cannot see the bits of a NaN.
    f64::NAN
  } else {
    let (n, y0, y1) = rem_pio2(x);
    match n & 3 {
      0 => kernel_sin(y0, y1, 1),
      1 => kernel_cos(y0, y1),
      2 => -kernel_sin(y0, y1, 1),
      _ => -kernel_cos(y0, y1),
    }
  }
}

/// The cosine of `x`.
pub(in super::super) fn cos(x: f64) -> f64 {
  let ix = abs_high_word(x);
  if ix <= PIO4_HIGH {
    kernel_cos(x, 0.0)
  } else if ix >= INFINITY_HIGH {
    // The C code returns `x - x`. JavaScript cannot see the bits of a NaN.
    f64::NAN
  } else {
    let (n, y0, y1) = rem_pio2(x);
    match n & 3 {
      0 => kernel_cos(y0, y1),
      1 => -kernel_sin(y0, y1, 1),
      2 => -kernel_cos(y0, y1),
      _ => kernel_sin(y0, y1, 1),
    }
  }
}

/// The tangent of `x`.
pub(in super::super) fn tan(x: f64) -> f64 {
  let ix = abs_high_word(x);
  if ix <= PIO4_HIGH {
    kernel_tan(x, 0.0, 1)
  } else if ix >= INFINITY_HIGH {
    // The C code returns `x - x`. JavaScript cannot see the bits of a NaN.
    f64::NAN
  } else {
    // An even n gives tan(y), an odd n gives -1/tan(y).
    let (n, y0, y1) = rem_pio2(x);
    kernel_tan(y0, y1, 1 - ((n & 1) << 1))
  }
}

/// The sine of `x + y` for `|x| <= pi/4`, where `y` is the tail of `x`. `iy`
/// is 0 when `y` is zero.
fn kernel_sin(x: f64, y: f64, iy: i32) -> f64 {
  let ix = abs_high_word(x);
  // |x| < 2^-27. The C code tests `(int)x == 0`, which is always true here,
  // only to set the inexact flag.
  if ix < TWO_M27_HIGH {
    return x;
  }
  // sin(x + y) = x + (S1*x^3 + (x^2*(r - y/2) + y)), with
  // r = x^3*(S2 + x^2*(S3 + ...)).
  let z = x * x;
  let v = z * x;
  let r = S2 + z * (S3 + z * (S4 + z * (S5 + z * S6)));
  if iy == 0 {
    x + v * (S1 + z * r)
  } else {
    x - ((z * (HALF * y - v * r) - y) - v * S1)
  }
}

/// The cosine of `x + y` for `|x| <= pi/4`, where `y` is the tail of `x`.
fn kernel_cos(x: f64, y: f64) -> f64 {
  let ix = abs_high_word(x);
  // |x| < 2^-27. The C code tests `(int)x == 0`, which is always true here,
  // only to set the inexact flag.
  if ix < TWO_M27_HIGH {
    return ONE;
  }
  // cos(x + y) = 1 - (x^2/2 - (r - x*y)), with r = C1*x^4 + ... + C6*x^14.
  let z = x * x;
  let r = z * (C1 + z * (C2 + z * (C3 + z * (C4 + z * (C5 + z * C6)))));
  // |x| < 0.3.
  if ix < 0x3FD3_3333 {
    return ONE - (0.5 * z - (z * r - x * y));
  }
  // For a larger x, subtract qx = x/4 (or 0.28125 when x > 0.78125) from both
  // sides. 1 - qx and x^2/2 - qx are exact, so the result loses less.
  let qx = if ix > 0x3FE9_0000 {
    0.28125
  } else {
    from_words(ix - 0x0020_0000, 0)
  };
  let iz = 0.5 * z - qx;
  let a = ONE - qx;
  a - (iz - (z * r - x * y))
}

/// `tan(x + y)` when `iy` is 1, or `-1/tan(x + y)` when `iy` is -1, for
/// `|x| <= pi/4`, where `y` is the tail of `x`.
fn kernel_tan(x: f64, y: f64, iy: i32) -> f64 {
  let hx = signed_high_word(x);
  let ix = without_sign(hx);
  // |x| < 2^-28. The C code tests `(int)x == 0`, which is always true here,
  // only to set the inexact flag.
  if ix < TWO_M28_HIGH as i32 {
    // The C code returns 1/|x| when x is zero and iy is -1. iy is -1 only for
    // an x from `rem_pio2`, and that x is never zero. No double is closer than
    // 2^-61 to a multiple of pi/2. The closest is 6381956970095103 * 2^797.
    if iy == 1 {
      return x;
    }
    return minus_reciprocal(x, y);
  }

  // |x| >= 0.6744: tan(x) = tan(pi/4 - y') = 1 - 2*(tan(y') - tan(y')^2/(1 +
  // tan(y'))), where y' = pi/4 - |x|.
  let (mut x, mut y) = (x, y);
  if ix >= 0x3FE5_9428 {
    if hx < 0 {
      x = -x;
      y = -y;
    }
    let z = PIO4 - x;
    let w = PIO4LO - y;
    x = z + w;
    y = 0.0;
  }

  // tan(x + y) = x + (T[0]*x^3 + (x^2*(r + y) + y)). The odd and the even
  // terms of r are two polynomials in x^4.
  let z = x * x;
  let w = z * z;
  let r = T[1] + w * (T[3] + w * (T[5] + w * (T[7] + w * (T[9] + w * T[11]))));
  let v = z * (T[2] + w * (T[4] + w * (T[6] + w * (T[8] + w * (T[10] + w * T[12])))));
  let s = z * x;
  let r = y + z * (s * (r + v) + y);
  let r = r + T[0] * s;
  let w = x + r;
  if ix >= 0x3FE5_9428 {
    let v = f64::from(iy);
    return f64::from(1 - ((hx >> 30) & 2)) * (v - 2.0 * (x - (w * w / (w + v) - r)));
  }
  if iy == 1 {
    return w;
  }
  minus_reciprocal(x, r)
}

/// `-1 / (x + tail)`, with more accuracy than the division alone.
///
/// Let `z` and `t` be `x + tail` and `-1 / (x + tail)` with the low word
/// cleared. Then `z * t` is exact, and one correction step fixes `t`.
fn minus_reciprocal(x: f64, tail: f64) -> f64 {
  let w = x + tail;
  let z = with_low_word(w, 0);
  let v = tail - (z - x);
  let a = -ONE / w;
  let t = with_low_word(a, 0);
  let s = ONE + t * z;
  t + a * (s + t * v)
}

#[cfg(test)]
#[path = "tests/trig_tests.rs"]
mod trig_tests;
