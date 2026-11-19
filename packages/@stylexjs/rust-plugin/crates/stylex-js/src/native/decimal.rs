//! Binary64 decimal formatting with ECMAScript's larger-coefficient tie rounding.
use super::{Budget, value::number_string};
fn multiply(digits: &mut Vec<u8>, by: u16) {
  let mut carry = 0;
  for d in digits.iter_mut().rev() {
    let n = *d as u16 * by + carry;
    *d = (n % 10) as u8;
    carry = n / 10;
  }
  while carry > 0 {
    digits.insert(0, (carry % 10) as u8);
    carry /= 10;
  }
}
fn exact(n: f64, budget: &Budget) -> Option<(Vec<u8>, i32)> {
  if n == 0.0 {
    return Some((vec![0], 0));
  }
  let bits = n.abs().to_bits();
  let exp = ((bits >> 52) & 2047) as i32;
  let frac = bits & ((1u64 << 52) - 1);
  let (mant, shift) = if exp == 0 {
    (frac, -1074)
  } else {
    (frac | (1u64 << 52), exp - 1023 - 52)
  };
  let mut digits = mant
    .to_string()
    .bytes()
    .map(|c| c - b'0')
    .collect::<Vec<_>>();
  for _ in 0..shift.unsigned_abs() {
    budget.spend(digits.len())?;
    multiply(&mut digits, if shift < 0 { 5 } else { 2 });
  }
  let scale = if shift < 0 { -shift } else { 0 };
  let exponent = digits.len() as i32 - scale - 1;
  Some((digits, exponent))
}
fn rounded(n: f64, precision: usize, budget: &Budget) -> Option<(String, i32)> {
  let (mut digits, mut exp) = exact(n, budget)?;
  let up = digits.get(precision).is_some_and(|d| *d >= 5);
  digits.truncate(precision);
  digits.resize(precision, 0);
  if up {
    let mut i = precision;
    while i > 0 && digits[i - 1] == 9 {
      digits[i - 1] = 0;
      i -= 1;
    }
    if i > 0 {
      digits[i - 1] += 1;
    } else {
      digits[0] = 1;
      exp += 1;
    }
  }
  Some((
    digits.into_iter().map(|d| char::from(b'0' + d)).collect(),
    exp,
  ))
}
fn scientific(digits: &str, exp: i32) -> String {
  let frac = if digits.len() > 1 {
    format!(".{}", &digits[1..])
  } else {
    String::new()
  };
  format!(
    "{}{frac}e{}{exp}",
    &digits[..1],
    if exp >= 0 { "+" } else { "" }
  )
}
fn shortest(n: f64) -> (String, i32) {
  let text = number_string(n.abs());
  let (base, e) = text
    .split_once('e')
    .map(|(b, e)| (b, e.parse::<i32>().unwrap()))
    .unwrap_or((&text, 0));
  let point = base.find('.').unwrap_or(base.len());
  let raw = base
    .bytes()
    .filter(|b| *b != b'.')
    .map(char::from)
    .collect::<String>();
  let leading = raw.bytes().take_while(|b| *b == b'0').count();
  if leading == raw.len() {
    return ("0".into(), 0);
  }
  let digits = raw[leading..].trim_end_matches('0').to_owned();
  let exp = point as i32 - leading as i32 - 1 + e;
  (digits, exp)
}
pub(super) fn exponential(n: f64, fraction: Option<usize>, budget: &Budget) -> Option<String> {
  if !n.is_finite() {
    return Some(number_string(n));
  }
  let (digits, exp) = if let Some(f) = fraction {
    rounded(n, f + 1, budget)?
  } else {
    shortest(n)
  };
  Some(format!(
    "{}{}",
    if n < 0.0 { "-" } else { "" },
    scientific(&digits, exp)
  ))
}
pub(super) fn precision(n: f64, p: usize, budget: &Budget) -> Option<String> {
  if !n.is_finite() {
    return Some(number_string(n));
  }
  let (digits, exp) = rounded(n, p, budget)?;
  let body = if exp < -6 || exp >= p as i32 {
    scientific(&digits, exp)
  } else if exp < 0 {
    format!("0.{}{}", "0".repeat((-exp - 1) as usize), digits)
  } else {
    let whole = exp as usize + 1;
    if whole >= digits.len() {
      format!("{}{}", digits, "0".repeat(whole - digits.len()))
    } else {
      format!("{}.{}", &digits[..whole], &digits[whole..])
    }
  };
  Some(format!("{}{}", if n < 0.0 { "-" } else { "" }, body))
}
