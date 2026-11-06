//! Expectations are what `@stylexjs/babel-plugin` emits for the same declaration. Author-facing
//! cases are asserted on `transform_value` in `stylex-transform`.

use crate::values::content::transform_content_value;

// ── a value that is already CSS ──────────────────────────────────────

#[test]
fn passes_through_a_string_list_a_separator_stands_in() {
  // A separator is not a component.
  let input = "\"icon\" / \"Alt text\"";

  assert_eq!(transform_content_value(input), input);
}

#[test]
fn passes_through_a_string_beside_a_function_it_does_not_know() {
  // Admitted as a closed function, not by name.
  assert_eq!(transform_content_value("\"a\" foo(1)"), "\"a\" foo(1)");
}

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

#[test]
fn quotes_keywords_that_no_string_stands_beside() {
  // A list of keywords alone is not the author writing a string.
  assert_eq!(
    transform_content_value("open-quote close-quote"),
    "\"open-quote close-quote\""
  );
}

// ── escaping ─────────────────────────────────────────────────────────

#[test]
fn writes_every_line_break_as_one_escape() {
  let cases = [
    ("a\nb", "\"a\\A b\""),
    ("a\rb", "\"a\\A b\""),
    // A CRLF is one line break and earns one escape.
    ("a\r\nb", "\"a\\A b\""),
    ("a\u{c}b", "\"a\\A b\""),
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
fn passes_through_a_component_list_of_five_thousand_strings() {
  let input = vec!["\"x\""; 5_000].join(" ");

  assert_eq!(transform_content_value(&input), input);
}

#[test]
fn quotes_a_very_long_text_value_once() {
  let text = "He said \"hi\" and left. ".repeat(2_000);

  // The whole answer, not just a quote count.
  let expected = format!("\"{}\"", text.replace('"', "\\\""));

  assert_eq!(transform_content_value(&text), expected);
}

#[test]
fn reads_a_deeply_nested_function_as_one_component() {
  let nested = format!("{}{}", "calc(".repeat(500), ")".repeat(500));
  let input = format!("\"a\" {}", nested);

  assert_eq!(transform_content_value(&input), input);
}

#[test]
fn quotes_a_lone_quote_character() {
  assert_eq!(transform_content_value("\""), "\"\\\"\"");
}

#[test]
fn quotes_an_empty_value() {
  assert_eq!(transform_content_value(""), "\"\"");
}
