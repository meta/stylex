use std::fmt;

/// Reads a leading number out of `input` exactly as JS `parseFloat` does,
/// returning `None` where JS returns `NaN`. See `CONTEXT.md` for why the
/// spelling has to match JS rather than merely round-trip.
///
/// Rust's `str::parse::<f64>` is not a substitute: it rejects any string with
/// trailing characters, where this reads as much of a leading number as it can
/// and ignores the rest — which is the whole point for CSS, where `"10px"` has
/// to yield `10`. [`to_js_string`] is the complement.
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

  // An exponent joins the literal only once it is complete: `1e` and `1e+` are
  // the longest-prefix rule at work, and both read back as `1`.
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

  // The prefix is a decimal literal by construction, and every spelling this
  // scan admits is one `f64` also accepts, so the error arm never fires in
  // practice — reported as failure rather than unwrapped, per the crate rules.
  rest[..end].parse::<f64>().ok()
}

/// The whitespace the language skips around a numeric literal: ECMA-262's
/// `StrWhiteSpace`, which is the Unicode space separators plus the line
/// terminators and the byte-order mark.
///
/// Hand-rolled rather than `char::is_whitespace`, which disagrees at both ends:
/// it admits U+0085, which JS does not skip, and omits U+FEFF, which JS does.
///
/// One home for the set, because `parseFloat` and `StringToNumber` skip the
/// same characters and two copies of one reading are what let the two drift.
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
///
/// The primitive; [`to_js_string`] is this with a `String` to write into. Two
/// forms rather than one because the callers want different things: a `Display`
/// impl already holds a formatter, and `write!(f, "{}", to_js_string(x))` builds
/// a `String` only to copy it out and drop it on the next line. A `matrix3d()`
/// carries sixteen numbers, so that was sixteen allocations plus the `Vec` and
/// the `join` holding them.
///
/// Not allocation-free, and worth being exact about: the digit decomposition
/// below still allocates, because it reads the shortest round-tripping digits
/// back out of Rust's own `LowerExp`. What this removes is the allocation per
/// number *at the call site*, which is the one a formatter does not need. The
/// remaining two are inside the one function in this crate whose output feeds
/// the class-name hash directly, and rewriting it around a stack buffer would
/// put that at risk to save an allocation nobody has measured -- which
/// `guidelines/PERFORMANCE.md` is explicit about not doing.
///
/// Rust's `f64` `Display` is not a substitute: it never switches to
/// exponential form, so `1e21` would render as `"1000000000000000000000"` where
/// JS renders `"1e+21"`. Since this rendering reaches both generated code and
/// the class-name hash, the spelling itself is observable and has to match, not
/// merely round-trip to the same `f64`.
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

  if value < 0.0 {
    out.write_char('-')?;
  }

  // `s` and `n` are ECMA-262's `Number::toString` variables: `s` is the shortest
  // digit string that round-trips, and the value is `s × 10^(n - k)` where `k`
  // is the digit count. Rust's `LowerExp` emits both, as `d.ddde±x`.
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

/// Writes `values` into `out` as a comma-separated list, each spelled as JS
/// spells it.
///
/// One writer because three `Display` impls needed the same list and each spelled
/// it differently -- `matrix()` as six positional format arguments, `matrix3d()`
/// and `linear()` each as a `Vec<String>` and a `join`. The list is what they
/// have in common, and the two `join` forms were also the two that allocated once
/// per number and once more for the result.
///
/// The separator is `", "` and not a parameter. Every caller is a CSS function's
/// argument list, which is the only shape this is for, and a parameter with one
/// value across every call site is a choice a reader has to check rather than a
/// choice anything makes. Give it one when a second separator turns up.
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
///
/// [`write_js_number`] with a `String` to write into, and the form most callers
/// want. Use the other one from a `Display` impl, where the formatter is already
/// to hand.
pub fn to_js_string(value: f64) -> String {
  let mut result = String::with_capacity(24);

  // Discarded rather than matched. `fmt::Write for String` returns `Ok`
  // unconditionally -- pushing onto a `String` has no failure mode -- so an
  // `Err` arm here is a branch no test can reach, and this repo does not exclude
  // code from coverage. A `match` was tried for the letter of `RUST.md`'s
  // "handle all cases" rule and reverted: the rule is about `.unwrap()` and
  // `.expect()`, neither of which this is, and trading an unreachable arm for it
  // makes the coverage gate the thing that is wrong rather than the code.
  //
  // The propagation itself *is* tested, one level down:
  // `write_js_number` is driven with a writer that refuses at each position in
  // turn, so every `?` in it is exercised.
  let _ = write_js_number(&mut result, value);

  result
}

/// Decomposes a finite, strictly positive `f64` into ECMA-262's `s` (the
/// shortest round-tripping digit string) and `n` (the decimal exponent, such
/// that the value is `s × 10^(n - s.len())`).
///
/// Rust's `LowerExp` already picks the same shortest digits as JS, so this only
/// has to re-read them out of `d.ddde±x`. The scan is hand-rolled rather than
/// `split_once` + `parse` so that every branch is reachable for some input and
/// no unreachable error path is left behind.
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
      // `to_digit` rather than `ch as u8 - b'0'`: the subtraction underflows,
      // and so panics in a debug build, for any non-digit that reaches this
      // arm. Nothing `LowerExp` emits today would, but the guarantee that it
      // never will is not this crate's to make.
      //
      // Reading a non-digit as a zero is the release behaviour, because a class
      // name spelled from a slightly wrong exponent beats aborting a compile.
      // The `debug_assert` is what keeps that from being silent: this function
      // exists to match `Number.prototype.toString` exactly, so a wrong
      // exponent is a wrong class name, and the test suite should say so
      // rather than the divergence being found in a stylesheet.
      //
      // Asserted on the character rather than written as a second match arm,
      // and carrying no formatted message, for the reason `hash::to_radix`
      // gives: both would leave regions behind that no input can reach and no
      // test can cover.
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

#[cfg(test)]
#[path = "tests/number_test.rs"]
mod tests;
