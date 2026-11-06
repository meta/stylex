//! Splitting a word into a number and a unit.

const MINUS: u32 = b'-' as u32;
const PLUS: u32 = b'+' as u32;
const DOT: u32 = b'.' as u32;
const EXP: u32 = b'e' as u32;
const EXP_UPPER: u32 = b'E' as u32;

/// Stand-in for JS `NaN` past the end of the string; see `parse::char_code_at`.
const OUT_OF_RANGE: u32 = u32::MAX;

fn char_code_at(value: &[u8], pos: usize) -> u32 {
  match value.get(pos) {
    Some(byte) => u32::from(*byte),
    None => OUT_OF_RANGE,
  }
}

fn is_digit(code: u32) -> bool {
  (48..=57).contains(&code)
}

/// A word split into its leading number and whatever follows (`10zz` → `("10", "zz")`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Dimension<'value> {
  /// The leading number, as written.
  pub number: &'value str,
  /// The rest; empty for a bare number.
  pub unit: &'value str,
}

/// Whether three code points would start a number, per
/// <https://www.w3.org/TR/css-syntax-3/#starts-with-a-number>.
fn like_number(value: &[u8]) -> bool {
  let code = char_code_at(value, 0);

  if code == PLUS || code == MINUS {
    let next_code = char_code_at(value, 1);

    if is_digit(next_code) {
      return true;
    }

    return next_code == DOT && is_digit(char_code_at(value, 2));
  }

  if code == DOT {
    return is_digit(char_code_at(value, 1));
  }

  is_digit(code)
}

/// Splits a word into its leading number and unit, per
/// <https://www.w3.org/TR/css-syntax-3/#consume-number>; `None` if it doesn't start with one.
pub fn unit(value: &str) -> Option<Dimension<'_>> {
  let bytes = value.as_bytes();
  let length = bytes.len();
  let mut pos = 0;

  if length == 0 || !like_number(bytes) {
    return None;
  }

  let code = char_code_at(bytes, pos);

  if code == PLUS || code == MINUS {
    pos += 1;
  }

  while pos < length && is_digit(char_code_at(bytes, pos)) {
    pos += 1;
  }

  if char_code_at(bytes, pos) == DOT && is_digit(char_code_at(bytes, pos + 1)) {
    pos += 2;

    while pos < length && is_digit(char_code_at(bytes, pos)) {
      pos += 1;
    }
  }

  let code = char_code_at(bytes, pos);
  let next_code = char_code_at(bytes, pos + 1);
  let next_next_code = char_code_at(bytes, pos + 2);

  // An exponent joins only when complete: `1e+` splits as `("1", "e+")`.
  if (code == EXP || code == EXP_UPPER)
    && (is_digit(next_code)
      || ((next_code == PLUS || next_code == MINUS) && is_digit(next_next_code)))
  {
    pos += match next_code == PLUS || next_code == MINUS {
      true => 3,
      false => 2,
    };

    while pos < length && is_digit(char_code_at(bytes, pos)) {
      pos += 1;
    }
  }

  // Cuts are always on character boundaries; the fallbacks mirror JS `slice`.
  Some(Dimension {
    number: value.get(..pos).unwrap_or(value),
    unit: value.get(pos..).unwrap_or(""),
  })
}
