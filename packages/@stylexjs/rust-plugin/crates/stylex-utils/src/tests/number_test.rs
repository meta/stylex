use crate::number::{parse_js_float, to_js_string, write_js_number, write_js_number_list};
use std::fmt;

include!("number_parse_float_cases.rs");

/// Every expectation is the literal output of JS `String(value)`, which reaches
/// both generated code and the class-name hash.
#[test]
fn matches_js_string_number() {
  let cases: &[(f64, &str)] = &[
    (0.0, "0"),
    (-0.0, "0"),
    (1.0, "1"),
    (-1.0, "-1"),
    (42.5, "42.5"),
    (100.0, "100"),
    (0.1, "0.1"),
    (1234.5678, "1234.5678"),
    (1e6, "1000000"),
    (1.0 / 3.0, "0.3333333333333333"),
    // Boundary of the fixed-notation range: `n <= 21` stays expanded.
    (1e20, "100000000000000000000"),
    (123456789012345680000.0, "123456789012345680000"),
    (1e21, "1e+21"),
    (-1e21, "-1e+21"),
    (1.0000000000000001e21, "1.0000000000000001e+21"),
    // Boundary of the small-number range: `n > -6` stays expanded.
    (1e-6, "0.000001"),
    (1e-7, "1e-7"),
    (-1e-7, "-1e-7"),
    (2e-10, "2e-10"),
    (5e-324, "5e-324"),
    (f64::MAX, "1.7976931348623157e+308"),
  ];

  for (value, expected) in cases {
    assert_eq!(to_js_string(*value), *expected, "String({})", expected);
  }
}

#[test]
fn renders_non_finite_values() {
  assert_eq!(to_js_string(f64::NAN), "NaN");
  assert_eq!(to_js_string(f64::INFINITY), "Infinity");
  assert_eq!(to_js_string(f64::NEG_INFINITY), "-Infinity");
}

/// Compared bit-for-bit so that `-0` cannot pass as `0`: the two are equal under
/// `==` but spell differently once a caller does arithmetic on them.
#[test]
fn should_match_js_parse_float_on_every_generated_case() {
  for (input, expected) in PARSE_FLOAT_CASES {
    let actual = parse_js_float(input);

    match (actual, expected) {
      (Some(actual), Some(expected)) => assert_eq!(
        actual.to_bits(),
        expected.to_bits(),
        "parseFloat({:?}) is {}, got {}",
        input,
        expected,
        actual
      ),
      (None, None) => {},
      _ => panic!(
        "parseFloat({:?}) is {:?}, got {:?}",
        input, expected, actual
      ),
    }
  }
}

/// The failure case has to be visible in the type, so a caller cannot mistake a
/// legitimate zero for "no number here" the way a NaN sentinel invites.
#[test]
fn should_report_failure_rather_than_a_sentinel_value() {
  assert_eq!(parse_js_float("auto"), None);
  assert_eq!(parse_js_float("0"), Some(0.0));
}

/// Magnitudes past what an `f64` holds saturate rather than fail, the way JS
/// does. Spelled as repetition rather than as table entries because the inputs
/// run to hundreds of characters, and the expectations are categorical — no
/// digit of the result can be subtly wrong.
#[test]
fn should_saturate_rather_than_fail_at_the_magnitude_limits() {
  assert_eq!(parse_js_float(&"9".repeat(400)), Some(f64::INFINITY));
  assert_eq!(parse_js_float(&"1".repeat(320)), Some(f64::INFINITY));
  assert_eq!(
    parse_js_float(&format!("-{}", "9".repeat(400))),
    Some(f64::NEG_INFINITY)
  );
  assert_eq!(parse_js_float("1e99999999999"), Some(f64::INFINITY));

  // Underflow goes the other way, to a signed zero rather than to failure.
  assert_eq!(
    parse_js_float(&format!("0.{}1", "0".repeat(400))),
    Some(0.0)
  );
  assert_eq!(parse_js_float("1e-99999999999"), Some(0.0));

  // An exponent whose digit run is itself enormous still reads as its value.
  assert_eq!(
    parse_js_float(&format!("1e{}5", "0".repeat(300))),
    Some(100000.0)
  );
}

/// Whatever `to_js_string` spells, `parse_js_float` reads back unchanged. The
/// two are applied to the same number in sequence by the normalizers, so a
/// disagreement between them is a value that changes just by passing through.
#[test]
fn should_round_trip_every_value_to_js_string_can_spell() {
  let values: &[f64] = &[
    0.0,
    1.0,
    -1.0,
    0.1,
    -0.5,
    42.5,
    1e-7,
    1e21,
    -1e21,
    1.0 / 3.0,
    5e-324,
    f64::MAX,
    f64::MIN_POSITIVE,
    f64::INFINITY,
    f64::NEG_INFINITY,
  ];

  for value in values {
    let spelled = to_js_string(*value);
    let read_back = parse_js_float(&spelled);

    assert_eq!(
      read_back.map(f64::to_bits),
      Some(value.to_bits()),
      "{} spells as {:?}, which read back as {:?}",
      value,
      spelled,
      read_back
    );
  }
}

/// `-0` is the one value the round trip does not preserve, and that is correct
/// rather than a gap: JS `String(-0)` is `"0"`, so the sign is gone before
/// anything reads it back. Pinned because a future "fix" restoring the sign
/// here would diverge from the reference compiler on a value CSS really carries
/// — `scale(-0)` and `translateX(-0px)` both reach normalization.
#[test]
fn should_lose_the_sign_of_negative_zero_exactly_as_js_does() {
  let spelled = to_js_string(-0.0);
  assert_eq!(spelled, "0");

  let read_back = parse_js_float(&spelled);
  assert_eq!(read_back.map(f64::to_bits), Some(0.0_f64.to_bits()));
  assert_ne!(read_back.map(f64::to_bits), Some((-0.0_f64).to_bits()));

  // Read directly, though, the sign survives — it is the spelling that drops it.
  assert_eq!(
    parse_js_float("-0").map(f64::to_bits),
    Some((-0.0_f64).to_bits())
  );
}

/// A suffix that cannot continue the number never changes the number read —
/// this is the whole reason the utility exists, since `10px` has to yield the
/// same `10` that `10` does.
#[test]
fn should_ignore_any_suffix_that_cannot_continue_the_number() {
  let suffixes = [
    "", "px", "%", "em", " ", "  10", ")", ",", ";", "/2", "😀", "é", "\u{0}", "\u{85}", "-", "+",
    ".", "e", "e+", "auto",
  ];

  for number in ["0", "1", "-2.5", ".5", "1e3", "-0"] {
    let bare = parse_js_float(number);

    for suffix in suffixes {
      let suffixed = parse_js_float(&format!("{}{}", number, suffix));

      assert_eq!(
        suffixed.map(f64::to_bits),
        bare.map(f64::to_bits),
        "{:?} and {:?} read as different numbers",
        number,
        format!("{}{}", number, suffix)
      );
    }
  }
}

/// Input that spells no number at all is rejected however long it runs and
/// whatever it is made of. The scan walks bytes, so a multi-byte character must
/// never be mistaken for the start of one.
#[test]
fn should_reject_input_that_spells_no_number() {
  let inputs = [
    String::new(),
    " ".repeat(1000),
    "\u{a0}\u{2028}\u{feff}\t\n ".repeat(100),
    "auto".repeat(500),
    "😀".repeat(500),
    "٥".repeat(100),
    "－".repeat(100),
    "-".repeat(100),
    ".".repeat(100),
    "+".repeat(100),
    "e".repeat(100),
    "--custom-property".to_string(),
    "\u{0}".repeat(50),
  ];

  for input in &inputs {
    assert_eq!(parse_js_float(input), None, "{:?} is not a number", input);
  }
}

/// Every prefix and every suffix of an adversarial corpus is fed through, which
/// is the cheap way to prove the scan cannot walk off either end of a string or
/// split a character while slicing the matched prefix.
#[test]
fn should_never_panic_on_any_slice_of_adversarial_input() {
  let corpus = [
    "-1.5e-2px",
    "\u{feff}\u{a0}+.5E+10%",
    "😀1.5é2",
    "Infinity",
    "-Infinitypx",
    "\u{0}\u{85}\u{2029}.e3",
    "٥５−5",
    "1e",
    "..5..",
  ];

  for input in corpus {
    for (start, _) in input.char_indices().chain([(input.len(), ' ')]) {
      for (end, _) in input[start..]
        .char_indices()
        .chain([(input.len() - start, ' ')])
      {
        // Panicking here fails the test; the value itself is not the point.
        let _ = parse_js_float(&input[start..start + end]);
      }
    }
  }
}

// ── Writing into a formatter that can fail ──────────────────────────

/// A `fmt::Write` that accepts `allow` writes and refuses every one after.
///
/// The point is the refusal, not the text. `write_js_number` writes a number in
/// several pieces -- a sign, a digit run, a decimal point, an exponent -- and
/// each piece is a `?` that has to carry a writer's failure back out rather than
/// panic, truncate, or return `Ok` on a buffer it never filled. A `String` can
/// never fail, so nothing that writes into one exercises those paths, and every
/// production caller writes into a `Formatter` that can.
struct RefusingWriter {
  allow: usize,
  written: String,
}

impl RefusingWriter {
  fn after(allow: usize) -> Self {
    Self {
      allow,
      written: String::new(),
    }
  }
}

impl fmt::Write for RefusingWriter {
  fn write_str(&mut self, text: &str) -> fmt::Result {
    if self.allow == 0 {
      return Err(fmt::Error);
    }

    self.allow -= 1;
    self.written.push_str(text);

    Ok(())
  }
}

/// Every `?` in `write_js_number` propagates the writer's failure.
///
/// Driven by refusing at each position in turn, over values chosen to reach a
/// different arm each: the three early returns, a negative sign, an integer with
/// trailing zeros, a fraction, a leading-zero fraction, and the exponential form
/// with both signs. Whatever position fails, the call must answer `Err` and must
/// not have written past the refusal.
#[test]
fn write_js_number_propagates_a_writer_failure_at_every_position() {
  let values: &[f64] = &[
    f64::NAN,
    f64::INFINITY,
    f64::NEG_INFINITY,
    0.0,
    -0.0,
    -1.5,
    100.0,
    1.5,
    // `n == 0`, so the zero-padding loop runs zero times...
    0.5,
    // ...and these run it once and five times, which is the only way the
    // `write_char('0')` inside it is reached at all.
    0.05,
    1e-6,
    1e21,
    // `n == -6` exactly, which is the *exponential* branch rather than the
    // padded one -- the bound is exclusive, and getting that backwards is how
    // the loop body came to be unexercised.
    1e-7,
    -1e21,
    // A negative exponent with a fractional mantissa, so the exponential branch
    // writes its `.`, its `e`, and a `-` sign rather than a `+`.
    -1.25e-9,
  ];

  for &value in values {
    // The number of writes a successful run takes, found by letting one
    // succeed -- so the loop below covers every prefix of it and one past.
    let mut counting = RefusingWriter::after(usize::MAX);
    assert!(write_js_number(&mut counting, value).is_ok());
    let writes = usize::MAX - counting.allow;

    for allow in 0..writes {
      let mut refusing = RefusingWriter::after(allow);

      assert!(
        write_js_number(&mut refusing, value).is_err(),
        "value {value} refused at write {allow} of {writes} should have failed"
      );
    }

    // And with every write allowed, it succeeds and spells the same thing
    // `to_js_string` does -- the two must not be able to disagree.
    let mut permissive = RefusingWriter::after(writes);
    assert!(write_js_number(&mut permissive, value).is_ok());
    assert_eq!(permissive.written, to_js_string(value));
  }
}

/// The list writer separates with `", "` and spells each member as JS does.
#[test]
fn write_js_number_list_joins_with_a_comma_and_a_space() {
  let mut out = String::new();
  write_js_number_list(&mut out, [1.0, -0.5, 1e21]).expect("a String cannot fail");

  assert_eq!(out, "1, -0.5, 1e+21");
}

/// One member carries no separator, and none is written before the first.
#[test]
fn write_js_number_list_writes_no_separator_before_the_first_member() {
  let mut out = String::new();
  write_js_number_list(&mut out, [42.0]).expect("a String cannot fail");

  assert_eq!(out, "42");
}

/// An empty list writes nothing and still succeeds, so a caller that opened a
/// function name emits `f()` rather than a stray separator.
#[test]
fn write_js_number_list_of_nothing_writes_nothing() {
  let mut out = String::from("matrix(");
  write_js_number_list(&mut out, []).expect("a String cannot fail");
  out.push(')');

  assert_eq!(out, "matrix()");
}

/// The list writer propagates a failure from either of its two writes -- the
/// separator, and the number.
#[test]
fn write_js_number_list_propagates_a_writer_failure() {
  let values = [1.0, 2.0, 3.0];

  let mut counting = RefusingWriter::after(usize::MAX);
  assert!(write_js_number_list(&mut counting, values).is_ok());
  let writes = usize::MAX - counting.allow;
  assert!(
    writes > values.len(),
    "separators should add writes of their own"
  );

  for allow in 0..writes {
    let mut refusing = RefusingWriter::after(allow);

    assert!(
      write_js_number_list(&mut refusing, values).is_err(),
      "refusing at write {allow} of {writes} should have failed"
    );
  }
}
