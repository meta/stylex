use std::fmt;

/// Reads the leading number in `input` like JS `parseFloat`, returning `None` where JS gives `NaN`.
/// Unlike `str::parse`, trailing characters are ignored, so `"10px"` yields `10`.
pub fn parse_js_float(input: &str) -> Option<f64> {
  let rest = input.trim_start_matches(is_js_whitespace);
  let bytes = rest.as_bytes();

  let negative = bytes.first() == Some(&b'-');
  let mut end = usize::from(matches!(bytes.first(), Some(b'+' | b'-')));

  if bytes
    .get(end..)
    .is_some_and(|after_sign| after_sign.starts_with(b"Infinity"))
  {
    return Some(if negative {
      f64::NEG_INFINITY
    } else {
      f64::INFINITY
    });
  }

  let integral_end = skip_ascii_digits(bytes, end);
  let mut has_digits = integral_end > end;
  end = integral_end;

  if bytes.get(end) == Some(&b'.') {
    let fraction_end = skip_ascii_digits(bytes, end + 1);
    has_digits = has_digits || fraction_end > end + 1;

    // A trailing dot stays in the literal (`1.`), but a lone dot never does.
    if has_digits {
      end = fraction_end;
    }
  }

  if !has_digits {
    return None;
  }

  // An incomplete exponent is left out, so `1e` and `1e+` read as `1`.
  if matches!(bytes.get(end), Some(b'e' | b'E')) {
    let mut exponent_start = end + 1;

    if matches!(bytes.get(exponent_start), Some(b'+' | b'-')) {
      exponent_start += 1;
    }

    let exponent_end = skip_ascii_digits(bytes, exponent_start);

    if exponent_end > exponent_start {
      end = exponent_end;
    }
  }

  // The scanned prefix is always a valid `f64` literal, so this never fails in practice.
  rest[..end].parse::<f64>().ok()
}

/// ECMA-262 `StrWhiteSpace`, the whitespace JS skips around a numeric literal.
/// Unlike `char::is_whitespace`, this excludes U+0085 and includes U+FEFF.
pub fn is_js_whitespace(ch: char) -> bool {
  match ch {
    // Space separators (Unicode `Zs`).
    '\u{20}' | '\u{a0}' | '\u{1680}' | '\u{202f}' | '\u{205f}' | '\u{3000}' => true,
    '\u{2000}'..='\u{200a}' => true,
    // Line terminators.
    '\u{a}' | '\u{d}' | '\u{2028}' | '\u{2029}' => true,
    // Tab, vertical tab, form feed, and the byte-order mark.
    '\u{9}' | '\u{b}' | '\u{c}' | '\u{feff}' => true,
    _ => false,
  }
}

/// Index of the first byte at or after `from` that is not an ASCII digit.
fn skip_ascii_digits(bytes: &[u8], from: usize) -> usize {
  let mut index = from;

  while matches!(bytes.get(index), Some(byte) if byte.is_ascii_digit()) {
    index += 1;
  }

  index
}

/// Writes an `f64` into `out` exactly as JS `String(Number)` spells it.
/// The exact spelling matters because it reaches generated code and the class-name hash.
pub fn write_js_number(out: &mut impl fmt::Write, value: f64) -> fmt::Result {
  if value.is_nan() {
    return out.write_str("NaN");
  }
  if value.is_infinite() {
    return out.write_str(if value > 0.0 { "Infinity" } else { "-Infinity" });
  }
  if value == 0.0 {
    // Covers `-0`, which JS also renders as `"0"`.
    return out.write_str("0");
  }

  // Integers below 2^53 are exact, so their decimal digits can be written directly.
  if value.fract() == 0.0 && value.abs() < 9_007_199_254_740_992.0 {
    let mut digits = [0u8; 20];
    let mut cursor = digits.len();
    let mut rest = value.abs() as u64;
    while rest > 0 {
      cursor -= 1;
      digits[cursor] = b'0' + (rest % 10) as u8;
      rest /= 10;
    }
    if value < 0.0 {
      out.write_char('-')?;
    }
    for digit in &digits[cursor..] {
      out.write_char(char::from(*digit))?;
    }
    return Ok(());
  }

  if value < 0.0 {
    out.write_char('-')?;
  }

  // ECMA-262 `Number::toString`: the value is `s × 10^(n - k)`, where `k` is the digit count.
  let (s, n) = shortest_digits_and_exponent(value.abs());
  let k = s.len() as i32;

  if k <= n && n <= 21 {
    out.write_str(&s)?;
    for _ in 0..(n - k) {
      out.write_char('0')?;
    }
  } else if 0 < n && n <= 21 {
    let (integral, fractional) = s.split_at(n as usize);
    out.write_str(integral)?;
    out.write_char('.')?;
    out.write_str(fractional)?;
  } else if -6 < n && n <= 0 {
    out.write_str("0.")?;
    for _ in 0..(-n) {
      out.write_char('0')?;
    }
    out.write_str(&s)?;
  } else {
    let (first, rest) = s.split_at(1);
    out.write_str(first)?;
    if !rest.is_empty() {
      out.write_char('.')?;
      out.write_str(rest)?;
    }
    out.write_char('e')?;
    out.write_char(if n >= 1 { '+' } else { '-' })?;
    write!(out, "{}", (n - 1).abs())?;
  }

  Ok(())
}

/// Writes `values` into `out` as a `", "`-separated list, each spelled as JS spells it.
pub fn write_js_number_list(
  out: &mut impl fmt::Write,
  values: impl IntoIterator<Item = f64>,
) -> fmt::Result {
  for (index, value) in values.into_iter().enumerate() {
    if index > 0 {
      out.write_str(", ")?;
    }

    write_js_number(out, value)?;
  }

  Ok(())
}

/// Renders an `f64` exactly as JS `String(Number)` does.
pub fn to_js_string(value: f64) -> String {
  let mut result = String::with_capacity(24);

  // Writing to a `String` cannot fail.
  let _ = write_js_number(&mut result, value);

  result
}

/// Splits a finite, positive `f64` into ECMA-262's shortest digits `s` and exponent `n`.
/// Rust's `LowerExp` picks the same shortest digits as JS, so this re-reads `d.ddde±x`.
fn shortest_digits_and_exponent(value: f64) -> (String, i32) {
  let formatted = format!("{:e}", value);

  let mut digits = String::with_capacity(17);
  let mut exponent = 0i32;
  let mut exponent_is_negative = false;
  let mut in_exponent = false;

  for ch in formatted.chars() {
    match ch {
      'e' => in_exponent = true,
      '.' => {},
      '-' => exponent_is_negative = true,
      // A non-digit reads as zero in release builds rather than panicking.
      _ if in_exponent => {
        debug_assert!(ch.is_ascii_digit());

        exponent = exponent * 10 + ch.to_digit(10).unwrap_or(0) as i32;
      },
      _ => digits.push(ch),
    }
  }

  if exponent_is_negative {
    exponent = -exponent;
  }

  // ECMA's `n` is one past the exponent of the leading digit.
  (digits, exponent + 1)
}
