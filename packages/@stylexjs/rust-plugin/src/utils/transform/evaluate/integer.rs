//! Exact integer-radix conversion, bounded to binary64's exponent range. Once
//! more than 1024 significant bits have accumulated the result is Infinity;
//! remaining digits are still validated, but cannot grow storage or arithmetic.
pub(super) fn parse_digits(text: &str, radix: u32) -> Option<f64> {
    if text.is_empty() || !(2..=36).contains(&radix) {
        return None;
    }
    let mut words = vec![0u32];
    let mut overflow = false;
    for c in text.chars() {
        let digit = c.to_digit(radix)?;
        if overflow {
            continue;
        }
        let mut carry = digit as u64;
        for w in &mut words {
            let n = *w as u64 * radix as u64 + carry;
            *w = n as u32;
            carry = n >> 32;
        }
        if carry > 0 {
            words.push(carry as u32);
        }
        let bits = (words.len() - 1) * 32 + (32 - words.last().unwrap().leading_zeros()) as usize;
        if bits > 1024 {
            overflow = true;
            words.clear();
        }
    }
    if overflow {
        return Some(f64::INFINITY);
    }
    let mut bits = (words.len() - 1) * 32 + (32 - words.last().unwrap().leading_zeros()) as usize;
    if bits == 0 {
        return Some(0.0);
    }
    let bit = |i: usize| (words[i / 32] >> (i % 32)) & 1;
    if bits <= 53 {
        let n = words.iter().rev().fold(0u64, |n, w| (n << 32) | *w as u64);
        return Some(n as f64);
    }
    let mut mant = 0u64;
    for i in (bits - 53..bits).rev() {
        mant = (mant << 1) | bit(i) as u64;
    }
    let guard = bit(bits - 54) != 0;
    let sticky = (0..bits - 54).any(|i| bit(i) != 0);
    if guard && (sticky || mant & 1 != 0) {
        mant += 1;
    }
    if mant == 1u64 << 53 {
        mant >>= 1;
        bits += 1;
    }
    if bits > 1024 {
        return Some(f64::INFINITY);
    }
    let exponent = (bits - 1 + 1023) as u64;
    Some(f64::from_bits(
        (exponent << 52) | (mant & ((1u64 << 52) - 1)),
    ))
}
