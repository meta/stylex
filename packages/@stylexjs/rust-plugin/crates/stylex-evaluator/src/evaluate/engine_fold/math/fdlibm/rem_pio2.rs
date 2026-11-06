//! `__ieee754_rem_pio2` and `__kernel_rem_pio2` from `ieee754.cc`.
//!
//! These functions find `n` and `y` such that `x = n * pi/2 + y` and
//! `|y| <= pi/4`. The trigonometric functions then need only a polynomial on
//! `[-pi/4, pi/4]`. `y` is the sum of a head and a tail, so that it keeps more
//! than 53 bits.

use super::{HALF, from_words, high_word, signed_high_word, without_sign, words};

/// The bits of `2/pi` after the binary point, 24 bits in each item.
const TWO_OVER_PI: [i32; 66] = [
  0xA2F983, 0x6E4E44, 0x1529FC, 0x2757D1, 0xF534DD, 0xC0DB62, 0x95993C, 0x439041, 0xFE5163,
  0xABDEBB, 0xC561B7, 0x246E3A, 0x424DD2, 0xE00649, 0x2EEA09, 0xD1921C, 0xFE1DEB, 0x1CB129,
  0xA73EE8, 0x8235F5, 0x2EBB44, 0x84E99C, 0x7026B4, 0x5F7E41, 0x3991D6, 0x398353, 0x39F49C,
  0x845F8B, 0xBDF928, 0x3B1FF8, 0x97FFDE, 0x05980F, 0xEF2F11, 0x8B5A0A, 0x6D1F6D, 0x367ECF,
  0x27CB09, 0xB74F46, 0x3F669E, 0x5FEA2D, 0x7527BA, 0xC7EBE5, 0xF17B3D, 0x0739F7, 0x8A5292,
  0xEA6BFB, 0x5FB11F, 0x8D5D08, 0x560330, 0x46FC7B, 0x6BABF0, 0xCFBC20, 0x9AF436, 0x1DA9E3,
  0x91615E, 0xE61B08, 0x659985, 0x5F14A0, 0x68408D, 0xFFD880, 0x4D7327, 0x310606, 0x1556CA,
  0x73A8C9, 0x60E27B, 0xC08C6B,
];

/// The high word of `n * pi/2` for `n` from 1 to 32.
const NPIO2_HW: [i32; 32] = [
  0x3FF921FB, 0x400921FB, 0x4012D97C, 0x401921FB, 0x401F6A7A, 0x4022D97C, 0x4025FDBB, 0x402921FB,
  0x402C463A, 0x402F6A7A, 0x4031475C, 0x4032D97C, 0x40346B9C, 0x4035FDBB, 0x40378FDB, 0x403921FB,
  0x403AB41B, 0x403C463A, 0x403DD85A, 0x403F6A7A, 0x40407E4C, 0x4041475C, 0x4042106C, 0x4042D97C,
  0x4043A28C, 0x40446B9C, 0x404534AC, 0x4045FDBB, 0x4046C6CB, 0x40478FDB, 0x404858EB, 0x404921FB,
];

/// `pi/2` cut into pieces of 24 bits.
const PIO2: [f64; 8] = [
  f64::from_bits(0x3FF9_21FB_4000_0000),
  f64::from_bits(0x3E74_442D_0000_0000),
  f64::from_bits(0x3CF8_4698_8000_0000),
  f64::from_bits(0x3B78_CC51_6000_0000),
  f64::from_bits(0x39F0_1B83_8000_0000),
  f64::from_bits(0x387A_2520_4000_0000),
  f64::from_bits(0x36E3_8222_8000_0000),
  f64::from_bits(0x3569_F31D_0000_0000),
];

/// 2^24, the size of one piece of the product.
const TWO24: f64 = f64::from_bits(0x4170_0000_0000_0000);
/// 2^-24.
const TWON24: f64 = f64::from_bits(0x3E70_0000_0000_0000);
/// 53 bits of `2/pi`.
const INVPIO2: f64 = f64::from_bits(0x3FE4_5F30_6DC9_C883);
/// The first 33 bits of `pi/2`.
const PIO2_1: f64 = f64::from_bits(0x3FF9_21FB_5440_0000);
/// `pi/2 - PIO2_1`.
const PIO2_1T: f64 = f64::from_bits(0x3DD0_B461_1A62_6331);
/// The second 33 bits of `pi/2`.
const PIO2_2: f64 = f64::from_bits(0x3DD0_B461_1A60_0000);
/// `pi/2 - (PIO2_1 + PIO2_2)`.
const PIO2_2T: f64 = f64::from_bits(0x3BA3_198A_2E03_7073);
/// The third 33 bits of `pi/2`.
const PIO2_3: f64 = f64::from_bits(0x3BA3_198A_2E00_0000);
/// `pi/2 - (PIO2_1 + PIO2_2 + PIO2_3)`.
const PIO2_3T: f64 = f64::from_bits(0x397B_839A_2520_49C1);

/// The number of terms of `2/pi` that the first pass of the product uses, less
/// one. This is `init_jk[prec]` for `prec = 2`, the value that
/// `__ieee754_rem_pio2` gives.
const JK: usize = 4;
/// The number of terms of `PIO2` that the last product uses, less one.
const JP: usize = JK;

/// `n` and the head and the tail of `y`, where `x = n * pi/2 + y`.
///
/// The caller does not give an `x` whose high word without the sign is at most
/// that of `pi/4` (0x3FE921FB), or an `x` that is not finite. Those `x` need no
/// reduction.
pub(super) fn rem_pio2(x: f64) -> (i32, f64, f64) {
  let hx = signed_high_word(x);
  let ix = without_sign(hx);

  // |x| < 3pi/4: n is 1 or -1.
  if ix < 0x4002_D97C {
    // Near pi/2, 33 + 53 bits of pi/2 are not enough. Use 33 + 33 + 53.
    return if hx > 0 {
      let z = x - PIO2_1;
      if ix != 0x3FF9_21FB {
        let y0 = z - PIO2_1T;
        (1, y0, (z - y0) - PIO2_1T)
      } else {
        let z = z - PIO2_2;
        let y0 = z - PIO2_2T;
        (1, y0, (z - y0) - PIO2_2T)
      }
    } else {
      let z = x + PIO2_1;
      if ix != 0x3FF9_21FB {
        let y0 = z + PIO2_1T;
        (-1, y0, (z - y0) + PIO2_1T)
      } else {
        let z = z + PIO2_2;
        let y0 = z + PIO2_2T;
        (-1, y0, (z - y0) + PIO2_2T)
      }
    };
  }

  // |x| <= 2^19 * pi/2: subtract n * pi/2 in two or three pieces.
  if ix <= 0x4139_21FB {
    let t = x.abs();
    // t * INVPIO2 + HALF < 2^19 + 1, so the conversion does not saturate.
    let n = (t * INVPIO2 + HALF) as i32;
    let fn_ = f64::from(n);
    let mut r = t - fn_ * PIO2_1;
    // The first pass is good to 85 bits.
    let mut w = fn_ * PIO2_1T;
    // t is at least 2.356 here, so t * 2/pi + 0.5 is more than 1.99 and n is
    // at least 1. So n - 1 is a valid index when n < 32.
    let y0 = if n < 32 && ix != NPIO2_HW[(n - 1) as usize] {
      // No cancellation is possible.
      r - w
    } else {
      // Count the bits that the subtraction cancelled. Use more bits of pi/2
      // when it cancelled too many.
      let j = ix >> 20;
      let mut y0 = r - w;
      let i = j - ((high_word(y0) >> 20) & 0x7FF) as i32;
      if i > 16 {
        // A second pass, good to 118 bits.
        let t = r;
        w = fn_ * PIO2_2;
        r = t - w;
        w = fn_ * PIO2_2T - ((t - r) - w);
        y0 = r - w;
        let i = j - ((high_word(y0) >> 20) & 0x7FF) as i32;
        if i > 49 {
          // A third pass, good to 151 bits. This is enough for all x.
          let t = r;
          w = fn_ * PIO2_3;
          r = t - w;
          w = fn_ * PIO2_3T - ((t - r) - w);
          y0 = r - w;
        }
      }
      y0
    };
    let y1 = (r - y0) - w;
    return if hx < 0 { (-n, -y0, -y1) } else { (n, y0, y1) };
  }

  // A large |x|. Cut |x| into three integers of 24 bits, x = (t0 + t1 * 2^-24 +
  // t2 * 2^-48) * 2^e0, and let `kernel_rem_pio2` multiply them by 2/pi.
  let (_, low) = words(x);
  let e0 = (ix >> 20) - 1046;
  // The scaled z has the exponent 23, so 2^23 <= z < 2^24.
  let mut z = from_words((ix - (e0 << 20)) as u32, low);
  let mut tx = [0.0; 3];
  for t in tx.iter_mut().take(2) {
    *t = f64::from(z as i32);
    z = (z - *t) * TWO24;
  }
  tx[2] = z;
  // Do not use the zero terms at the end. tx[0] >= 2^23, so it is never zero.
  let nx = tx.len() - tx.iter().rev().take_while(|&&t| t == 0.0).count();
  let (n, y0, y1) = kernel_rem_pio2(&tx[..nx], e0);
  if hx < 0 { (-n, -y0, -y1) } else { (n, y0, y1) }
}

/// `n mod 8` and the head and the tail of `y`, where `x * 2^e0 = n * pi/2 + y`.
///
/// `x` holds the integer pieces of 24 bits of a positive number. The function
/// multiplies them by only the bits of `2/pi` that change `n mod 8` and `y`.
/// `prec` is always 2 in `ieee754.cc`, so this port does only that case.
fn kernel_rem_pio2(x: &[f64], e0: i32) -> (i32, f64, f64) {
  let jx = x.len() - 1;
  // e0 >= -3, so e0 - 3 >= -6, and the division rounds to zero: jv >= 0. The
  // C code sets a negative jv to 0, but it cannot be negative here.
  let jv = (e0 - 3) / 24;
  // q0 < 3 is the exponent of q[0].
  let mut q0 = e0 - 24 * (jv + 1);
  let jv = jv as usize;

  // f[i] is the term of 2/pi that multiplies x[jx] in q[i].
  let mut f = [0.0; 20];
  for (i, fi) in f.iter_mut().enumerate().take(jx + JK + 1) {
    *fi = if i + jv < jx {
      0.0
    } else {
      f64::from(TWO_OVER_PI[i + jv - jx])
    };
  }

  // q[i] is a piece of the product, with the exponent q0 - 24 * i.
  let mut q = [0.0; 20];
  for (i, qi) in q.iter_mut().enumerate().take(JK + 1) {
    *qi = product_term(x, &f, i);
  }

  let mut jz = JK;
  let mut iq = [0_i32; 20];
  let (n, z, ih) = loop {
    // Cut q into integers of 24 bits in iq, the low piece first.
    let mut z = q[jz];
    for (i, j) in (1..=jz).rev().enumerate() {
      let fw = f64::from((TWON24 * z) as i32);
      iq[i] = (z - TWO24 * fw) as i32;
      z = q[j - 1] + fw;
    }

    // Find n mod 8. The scale by 2^q0 is exact.
    z *= pow2(q0);
    z -= 8.0 * (z * 0.125).floor();
    let mut n = z as i32;
    z -= f64::from(n);

    // ih > 0 when the fraction is 0.5 or more.
    let mut ih = 0;
    if q0 > 0 {
      // iq[jz - 1] holds the low bits of n.
      let i = iq[jz - 1] >> (24 - q0);
      n += i;
      iq[jz - 1] -= i << (24 - q0);
      ih = iq[jz - 1] >> (23 - q0);
    } else if q0 == 0 {
      ih = iq[jz - 1] >> 23;
    } else if z >= 0.5 {
      ih = 2;
    }

    if ih > 0 {
      // The fraction is 0.5 or more. Add 1 to n and use 1 - fraction.
      n += 1;
      let mut carry = 0;
      for iqi in iq.iter_mut().take(jz) {
        let j = *iqi;
        if carry == 0 {
          if j != 0 {
            carry = 1;
            *iqi = 0x100_0000 - j;
          }
        } else {
          *iqi = 0xFF_FFFF - j;
        }
      }
      if q0 > 0 {
        // Remove the bits of n. q0 is 1 or 2, so this mask is the mask of the
        // C code for each case.
        iq[jz - 1] &= 0xFF_FFFF >> q0;
      }
      if ih == 2 {
        // `carry` is 0 or 1, and z - 0 is z. So this is the C code
        // `if (carry != 0) z -= scalbn(one, q0);`.
        z = 1.0 - z;
        z -= f64::from(carry) * pow2(q0);
      }
    }

    // When the fraction is zero to here, use more terms of 2/pi.
    if z == 0.0 && iq[JK..jz].iter().fold(0, |j, &v| j | v) == 0 {
      let mut k = 1;
      while k <= JK && iq[JK - k] == 0 {
        k += 1;
      }
      for i in jz + 1..=jz + k {
        f[jx + i] = f64::from(TWO_OVER_PI[jv + i]);
        q[i] = product_term(x, &f, i);
      }
      jz += k;
      continue;
    }
    break (n, z, ih);
  };

  // Remove the zero pieces at the top. The loop above stops only when a piece
  // in iq[JK..jz] is not zero, so this loop stops at JK or above.
  if z == 0.0 {
    jz -= 1;
    q0 -= 24;
    while iq[jz] == 0 {
      jz -= 1;
      q0 -= 24;
    }
  } else {
    // Cut the fraction into pieces of 24 bits if necessary.
    let z = z * pow2(-q0);
    if z >= TWO24 {
      let fw = f64::from((TWON24 * z) as i32);
      iq[jz] = (z - TWO24 * fw) as i32;
      jz += 1;
      q0 += 24;
      iq[jz] = fw as i32;
    } else {
      iq[jz] = z as i32;
    }
  }

  // Change the integer pieces back to doubles.
  let mut fw = pow2(q0);
  for i in (0..=jz).rev() {
    q[i] = fw * f64::from(iq[i]);
    fw *= TWON24;
  }

  // fq = PIO2[0..=JP] * q[jz..=0].
  let mut fq = [0.0; 20];
  for i in (0..=jz).rev() {
    fq[jz - i] = PIO2
      .iter()
      .zip(&q[i..])
      .take(JP.min(jz - i) + 1)
      .fold(0.0, |fw, (p, qk)| fw + p * qk);
  }

  // Add fq into a head and a tail.
  let fw = fq[..=jz].iter().rev().fold(0.0, |fw, &v| fw + v);
  let y0 = if ih == 0 { fw } else { -fw };
  let fw = fq[1..=jz].iter().fold(fq[0] - fw, |fw, &v| fw + v);
  let y1 = if ih == 0 { fw } else { -fw };
  (n & 7, y0, y1)
}

/// The sum of `x[j] * f[jx + i - j]`, which is the piece `q[i]` of the product.
fn product_term(x: &[f64], f: &[f64; 20], i: usize) -> f64 {
  let jx = x.len() - 1;
  x.iter()
    .enumerate()
    .fold(0.0, |fw, (j, &xj)| fw + xj * f[jx + i - j])
}

/// `2^n`, for `n` in the range of normal exponents. A product with it is exact
/// unless the result is subnormal, so it is `scalbn` for the values here.
///
/// The callers pass `q0` or `-q0`. Both stay within a few hundred of zero for
/// every double, so `1023 + n` is a positive normal exponent.
fn pow2(n: i32) -> f64 {
  f64::from_bits(((1023 + n) as u64) << 52)
}

#[cfg(test)]
#[path = "tests/rem_pio2_tests.rs"]
mod rem_pio2_tests;
