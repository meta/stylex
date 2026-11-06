//! A port of the fdlibm functions that Node uses for the `Math` statics.
//!
//! V8 has its own copy of fdlibm in `src/base/ieee754.cc`, and the `Math`
//! statics in Node call it. Each function here does the same steps in the
//! same order, and never fuses a multiplication and an addition into one step.
//! So on every platform, it gives the bits that Node gives on x64.
//!
//! Node on arm64 can give a different last bit. Its C++ compiler fuses some
//! multiplications and additions of the C code. The port does not do this, so
//! a class name is the same on every host. ADR 0008 gives the reason.
//!
//! The C code reads and writes the two 32-bit halves of a double. The helpers
//! below do the same.
//!
//! Each constant is written as the bits in the comment of the C code, so the
//! port cannot round it differently.
//!
//! The C code has tests such as `huge + x > one` that only set the inexact
//! flag. JavaScript cannot see the flags. Where such a test is true for every
//! argument that reaches it, the port keeps only its result, and a comment at
//! that place gives the range.

mod cbrt;
mod exp_log;
mod hyperbolic;
mod inverse_trig;
mod rem_pio2;
mod trig;

pub(super) use cbrt::cbrt;
pub(super) use exp_log::{exp, expm1, log, log1p, log2, log10};
pub(super) use hyperbolic::{acosh, asinh, atanh, cosh, sinh, tanh};
pub(super) use inverse_trig::{acos, asin, atan, atan2};
pub(super) use trig::{cos, sin, tan};

/// One, which several ports use.
const ONE: f64 = f64::from_bits(0x3FF0_0000_0000_0000);
/// One half, which several ports use.
const HALF: f64 = f64::from_bits(0x3FE0_0000_0000_0000);
/// `1.0e+300`. Its square overflows to an infinity.
const HUGE: f64 = f64::from_bits(0x7E37_E43C_8800_759C);
/// `1.0e-300`. Its square underflows to zero.
const TINY: f64 = f64::from_bits(0x01A5_6E1F_C2F8_F359);
/// `2^54`. A subnormal number times this is a normal number.
const TWO54: f64 = f64::from_bits(0x4350_0000_0000_0000);
/// `pi/4`.
const PIO4: f64 = f64::from_bits(0x3FE9_21FB_5444_2D18);

/// The high word of an infinity. A high word without its sign that is at least
/// this belongs to an infinity or a NaN.
const INFINITY_HIGH: u32 = 0x7FF0_0000;
/// The sign bit of a high word.
const SIGN_MASK: u32 = 0x8000_0000;
/// The high word of `2^-27`. Below it, the ports of the sine and the arcsine
/// give `x`.
const TWO_M27_HIGH: u32 = 0x3E40_0000;
/// The high word of `2^-28`. Below it, several ports give `x` or `1 + x`.
const TWO_M28_HIGH: u32 = 0x3E30_0000;
/// The high word of `ln(MAX_VALUE)`, about 709.78. At or above it, `exp(x)`
/// can overflow.
const LN_MAX_HIGH: u32 = 0x4086_2E42;

// Some ports compare a signed high word, as the C code does. They cast the
// constants above with `as i32`, which keeps the value, because each one is
// below 2^31.

/// The high and the low 32 bits of `x`.
fn words(x: f64) -> (u32, u32) {
  let bits = x.to_bits();
  ((bits >> 32) as u32, bits as u32)
}

/// The high 32 bits of `x`: the sign, the exponent and the top of the
/// fraction.
fn high_word(x: f64) -> u32 {
  (x.to_bits() >> 32) as u32
}

/// The high word of `x` as the signed `int32_t` of the C code. It is negative
/// when the sign of `x` is set.
fn signed_high_word(x: f64) -> i32 {
  high_word(x) as i32
}

/// The two halves of `x`, with the high half signed as in the C code.
fn signed_words(x: f64) -> (i32, u32) {
  let (high, low) = words(x);
  (high as i32, low)
}

/// `ix` with bit 0 set when `low` is not zero, as the C code
/// `ix | ((lx | -lx) >> 31)` does. When `ix` is the high word of `|x|`, the
/// result is above that of an infinity only for NaN, and above 0x3FF00000 for
/// each `|x|` above 1.
fn high_with_low_flag(ix: i32, low: u32) -> u32 {
  ix as u32 | ((low | low.wrapping_neg()) >> 31)
}

/// The low 32 bits of `x`: the bottom of the fraction.
fn low_word(x: f64) -> u32 {
  x.to_bits() as u32
}

/// `x` with its high 32 bits replaced by `high`.
fn with_high_word(x: f64, high: u32) -> f64 {
  from_words(high, low_word(x))
}

/// `x` with its low 32 bits replaced by `low`.
fn with_low_word(x: f64, low: u32) -> f64 {
  f64::from_bits((x.to_bits() & 0xFFFF_FFFF_0000_0000) | u64::from(low))
}

/// The high 32 bits of `x` without the sign: the exponent and the top of the
/// fraction of `|x|`.
fn abs_high_word(x: f64) -> u32 {
  high_word(x) & !SIGN_MASK
}

/// The signed high word `high` without its sign, as the C code
/// `hx & 0x7fffffff` gives it. The result is never negative. This is the
/// signed form of [`abs_high_word`], for a port that keeps `hx` as well.
fn without_sign(high: i32) -> i32 {
  high & i32::MAX
}

/// The double with the high 32 bits `high` and the low 32 bits `low`.
fn from_words(high: u32, low: u32) -> f64 {
  f64::from_bits((u64::from(high) << 32) | u64::from(low))
}

#[cfg(test)]
#[path = "tests/test_support.rs"]
mod test_support;
