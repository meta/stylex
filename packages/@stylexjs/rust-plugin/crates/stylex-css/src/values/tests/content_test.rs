//! Expectations are what `@stylexjs/babel-plugin` emits for the same declaration. Author-facing
//! cases are asserted on `transform_value` in `stylex-transform`.

use crate::values::content::transform_content_value;

// ── a value that is already CSS ──────────────────────────────────────

// ── a value that is not ──────────────────────────────────────────────

#[test]
fn quotes_a_value_whose_components_are_not_all_content() {
  let cases = [
    ("\"a\" foo(", "\"\\\"a\\\" foo(\""),
    ("\"a\" contents", "\"\\\"a\\\" contents\""),
    ("\"a\"/*c*/", "\"\\\"a\\\"/*c*/\""),
    ("U+26 \"a\"", "\"U+26 \\\"a\\\"\""),
  ];

  for (input, expected) in cases {
    assert_eq!(
      transform_content_value(input),
      expected,
      "quoting `{}`",
      input
    );
  }
}

// ── escaping ─────────────────────────────────────────────────────────

#[test]
fn reads_a_backslash_as_the_start_of_an_escape() {
  let cases = [
    // `\e9` is a CSS escape; doubling its backslash would print it literally.
    ("caf\\e9 au lait", "\"caf\\e9 au lait\""),
    // An escaped line break continues the string; both CRLF characters are copied.
    ("trail\\\r\nx", "\"trail\\\r\nx\""),
    // A trailing backslash is the one backslash that is doubled.
    ("trail\\", "\"trail\\\\\""),
  ];

  for (input, expected) in cases {
    assert_eq!(
      transform_content_value(input),
      expected,
      "quoting `{:?}`",
      input
    );
  }
}

// ── large and awkward input ──────────────────────────────────────────

#[test]
fn reads_a_deeply_nested_function_as_one_component() {
  let nested = format!("{}{}", "calc(".repeat(500), ")".repeat(500));
  let input = format!("\"a\" {}", nested);

  assert_eq!(transform_content_value(&input), input);
}
