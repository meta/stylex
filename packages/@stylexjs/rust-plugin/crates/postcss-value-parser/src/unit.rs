//! Splitting a word into a number and a unit. See the crate documentation in
//! `lib.rs` for what it is and who holds its copyright.

const MINUS: u32 = b'-' as u32;
const PLUS: u32 = b'+' as u32;
const DOT: u32 = b'.' as u32;
const EXP: u32 = b'e' as u32;
const EXP_UPPER: u32 = b'E' as u32;

/// See `parse::char_code_at` — the same stand-in for reading past the end of
/// the string, chosen so that every comparison below fails the way `NaN` does.
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

/// A word split into the number it starts with and whatever follows.
///
/// The unit is whatever the number scan did not consume, whether or not it is a
/// real CSS unit: `10px` splits as `("10", "px")` and `10zz` as `("10", "zz")`.
///
/// Both halves borrow the word rather than owning copies of it. Four
/// normalizers ask this of every word in every value, and each only reads or
/// compares what comes back — building a replacement string, when there is one
/// to build, is a separate step that happens far less often.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Dimension<'value> {
  /// The leading number, spelled exactly as the author wrote it.
  pub number: &'value str,
  /// Everything after it, empty when the word is a bare number.
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

/// Splits a word into its leading number and its unit, per
/// <https://www.w3.org/TR/css-syntax-3/#consume-number>.
///
/// Returns `None` when the word does not start with a number at all — which is
/// how the normalizers tell `0px` apart from `auto`.
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

  // An exponent joins the number only when it is complete. `1e` and `1e+` split
  // as `("1", "e")` and `("1", "e+")`, because the exponent never happened.
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

  // Every cut lands after an ASCII digit, sign, dot or exponent marker, so the
  // split is always on a character boundary. The fallbacks are what JavaScript
  // `slice` would do with an out-of-range index — the whole word, then nothing
  // — rather than the empty split a `unwrap_or_default` on both halves would
  // invent, which reads as a successful parse of a number that is not there.
  Some(Dimension {
    number: value.get(..pos).unwrap_or(value),
    unit: value.get(pos..).unwrap_or(""),
  })
}
