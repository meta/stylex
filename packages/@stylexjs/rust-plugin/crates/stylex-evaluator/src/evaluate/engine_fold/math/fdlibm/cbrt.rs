//! `cbrt` from `ieee754.cc`.

use super::{INFINITY_HIGH, SIGN_MASK, TWO54, abs_high_word, from_words, words};

/// `(1023 - 1023/3 - 0.03306235651) * 2^20`.
const B1: u32 = 715_094_163;
/// `(1023 - 1023/3 - 54/3 - 0.03306235651) * 2^20`.
const B2: u32 = 696_219_795;

// |1/cbrt(x) - p(x)| < 2^-23.5 (~[-7.93e-8, 7.929e-8]).
const P0: f64 = f64::from_bits(0x3FFE_03E6_0F61_E692);
const P1: f64 = f64::from_bits(0xBFFE_28E0_92F0_2420);
const P2: f64 = f64::from_bits(0x3FF9_F160_4A49_D6C2);
const P3: f64 = f64::from_bits(0xBFE8_44CB_BEE7_51D9);
const P4: f64 = f64::from_bits(0x3FC2_B000_D4E4_EDD7);

/// The cube root of `x`, with an error below 0.667 units in the last place.
pub(in super::super) fn cbrt(x: f64) -> f64 {
  let (high, low) = words(x);
  let sign = high & SIGN_MASK;
  let hx = high ^ sign;

  // The cube root of NaN or an infinity is itself.
  if hx >= INFINITY_HIGH {
    return x + x;
  }

  // A rough cube root to 5 bits. Integer division of the exponent bits by 3
  // gives almost the cube root of the power of two. A zero or subnormal `x`
  // is first scaled by 2^54 so that its exponent bits are not zero.
  let rough = if hx < 0x0010_0000 {
    if (hx | low) == 0 {
      return x;
    }
    let scaled = TWO54 * x;
    from_words(sign | (abs_high_word(scaled) / 3 + B2), 0)
  } else {
    from_words(sign | (hx / 3 + B1), 0)
  };

  // A new cube root to 23 bits, from a polynomial in `rough^3 / x`.
  let r = (rough * rough) * (rough / x);
  let t = rough * ((P0 + r * (P1 + r * P2)) + ((r * r) * r) * (P3 + r * P4));

  // Round `t` away from zero to 23 bits, so that `t * t` below is exact.
  let t = f64::from_bits((t.to_bits() + 0x8000_0000) & 0xFFFF_FFFF_C000_0000);

  // One Newton step to 53 bits.
  let s = t * t;
  let r = x / s;
  let w = t + t;
  let r = (r - t) / (w + r);
  t + t * r
}

#[cfg(test)]
#[path = "tests/cbrt_tests.rs"]
mod cbrt_tests;
