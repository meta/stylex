//! Value normalization asserted against the reference compiler's own output.
//!
//! Enters through [`normalize_value`], not `normalize_css_property_value`, so the compiler's extra
//! guards (unclosed comment, nesting budget, terminators) are excluded; those are covered in
//! `value_normalization_parity_test.rs`. Reference-compiler defects are reproduced, not corrected:
//! hash parity outranks local correctness.

use std::panic::{AssertUnwindSafe, catch_unwind};

use stylex_constants::constants::messages::{
  LINT_IMPORTANT_NOT_LAST, LINT_UNCLOSED_FUNCTION, LINT_UNCLOSED_STRING, LINT_VALUE_HAS_NO_TOKENS,
};
use stylex_structures::stylex_state_options::StyleXStateOptions;

use crate::css::{
  common::MAX_VALUE_NESTING_DEPTH,
  normalize_value::normalize_value,
  tests::support::{default_options, panic_message, rem_enabled_options},
};

/// (value, property, reference-compiler declaration text).
type Case = (&'static str, &'static str, &'static str);

fn check(cases: &[Case]) {
  check_with(cases, &default_options());
}

fn check_with(cases: &[Case], options: &StyleXStateOptions) {
  for (value, key, expected) in cases {
    let actual = normalize_value(value, key, options);

    assert_eq!(&actual, expected, "normalizing `{key}: {value}`");
  }
}

/// Asserts `value` is rejected for `key` with a message containing `expected`.
fn rejects(value: &str, key: &str, expected: &str) {
  let options = default_options();
  let result = catch_unwind(AssertUnwindSafe(|| normalize_value(value, key, &options)));
  let message = panic_message(result);

  assert!(
    message.contains(expected),
    "expected `{key}: {value}` to be rejected with `{expected}`, got: {message}"
  );
}

// ── What no normalizer touches ──

#[test]
fn leaves_spellings_no_normalizer_has_an_opinion_about_alone() {
  check(&[
    ("#ffffff", "color", "#ffffff"),
    ("#FFFFFF", "color", "#FFFFFF"),
    ("#FfF", "color", "#FfF"),
    ("#ffffffff", "color", "#ffffffff"),
    ("RED", "color", "RED"),
    ("Red", "color", "Red"),
    ("rgb(from red r g b)", "color", "rgb(from red r g b)"),
    (
      "calc(-1 * var(--spacing))",
      "margin",
      "calc(-1 * var(--spacing))",
    ),
    (
      "'sidebar content'",
      "gridTemplateAreas",
      "'sidebar content'",
    ),
    (
      "\"sidebar content\"",
      "gridTemplateAreas",
      "\"sidebar content\"",
    ),
    ("'a' 'b'", "gridTemplateAreas", "'a' 'b'"),
    ("1px solid red", "border", "1px solid red"),
    ("auto", "width", "auto"),
    ("inherit", "width", "inherit"),
    ("100%", "width", "100%"),
    ("CALC(1PX + 2PX)", "width", "CALC(1PX + 2PX)"),
    ("Translate(0Px)", "transform", "Translate(0Px)"),
    ("calc(100%-10px)", "width", "calc(100%-10px)"),
    ("0 auto", "margin", "0 auto"),
    ("1fr 1fr", "gridTemplateColumns", "1fr 1fr"),
    ("counter(x)", "content", "counter(x)"),
    ("attr(data-x)", "content", "attr(data-x)"),
    ("\"a\" attr(b) \"c\"", "content", "\"a\" attr(b) \"c\""),
    ("calc(calc(calc(1px)))", "width", "calc(calc(calc(1px)))"),
    (
      "calc(1px + calc(2px + calc(3px + calc(4px))))",
      "width",
      "calc(1px + calc(2px + calc(3px + calc(4px))))",
    ),
  ]);
}

/// Which is why the unit-sensitive normalizers miss upper-case spellings.
#[test]
fn never_folds_letter_case_in_a_unit() {
  check(&[
    ("0PX", "width", "0"),
    ("0DEG", "transform", "0"),
    ("10MS", "transitionDuration", "10MS"),
    ("10Ms", "transitionDuration", "10Ms"),
    ("1E3px", "width", "1E3px"),
  ]);
}

// ── Whitespace and separators ──

#[test]
fn collapses_whitespace_without_moving_it() {
  check(&[
    ("a   b   c", "color", "a b c"),
    ("  a  ", "color", "a"),
    ("a\tb", "color", "a b"),
    ("a\nb", "color", "a b"),
    ("a\r\nb", "color", "a b"),
    ("calc( 100% - 10px )", "width", "calc(100% - 10px)"),
    ("clamp( 1px , 2px , 3px )", "width", "clamp(1px,2px,3px)"),
    ("var( --a , red )", "color", "var(--a,red)"),
  ]);
}

/// A comma loses surrounding spaces; every other separator gains one on both sides.
#[test]
fn spaces_separators_by_which_separator_it_is() {
  check(&[
    ("a , b", "color", "a,b"),
    ("a ,b", "color", "a,b"),
    ("a, b", "color", "a,b"),
    ("a,b", "color", "a,b"),
    ("a/b", "color", "a / b"),
    ("a / b", "color", "a / b"),
    ("a  /  b", "color", "a / b"),
    ("a : b", "color", "a : b"),
    ("a:b", "color", "a : b"),
  ]);
}

#[test]
fn strips_padding_inside_function_parentheses() {
  check(&[
    ("translate( 1px , 2px )", "transform", "translate(1px,2px)"),
    ("min(1px, 2px)", "width", "min(1px,2px)"),
    ("url( a.png )", "backgroundImage", "url(a.png)"),
    (
      "a(b(c(d(e(f(g(h(0px)))))))) ",
      "width",
      "a(b(c(d(e(f(g(h(0px))))))))",
    ),
  ]);
}

// ── The importance annotation ──

#[test]
fn removes_the_space_before_an_importance_annotation() {
  check(&[
    ("red !important", "color", "red!important"),
    ("red   !important", "color", "red!important"),
    ("red!important", "color", "red!important"),
    ("red !important ", "color", "red!important"),
    (" red !important", "color", "red!important"),
    (
      "1px solid red !important",
      "border",
      "1px solid red!important",
    ),
  ]);
}

/// The annotation is matched case-sensitively.
#[test]
fn leaves_an_annotation_with_nothing_to_remove_before_it() {
  check(&[
    ("!important", "color", "!important"),
    ("a,!important", "color", "a,!important"),
    ("red !IMPORTANT", "color", "red !IMPORTANT"),
  ]);
}

/// Matches Babel's defect: the index is from the annotation's sibling list but removal is from the
/// top-level list, so `a b` comes back joined.
#[test]
fn removes_an_unrelated_node_for_an_annotation_written_inside_a_function() {
  check(&[
    ("a b calc(x !important)", "color", "ab calc(x !important)"),
    ("a  b calc(x !important)", "color", "ab calc(x !important)"),
    ("calc(x !important)", "color", "calc(x !important)"),
    ("a b calc(!important)", "color", "a b calc(!important)"),
    // A separator at that index: the removal is skipped.
    ("a, b calc(x !important)", "color", "a,b calc(x !important)"),
    // The first annotation removes a space; the second lands on a word and gives up.
    (
      "a b c calc(x !important y !important)",
      "color",
      "ab c calc(x !important y !important)",
    ),
    (
      "a b calc(x !important y !important)",
      "color",
      "ab calc(x !important y !important)",
    ),
  ]);
}

#[test]
fn rejects_an_annotation_inside_a_function_that_is_not_the_last_token() {
  for value in ["a b calc(x !important) d", "calc(x !important) a b"] {
    rejects(value, "color", LINT_IMPORTANT_NOT_LAST);
  }
}

/// Babel removes from the list it is walking and reads past the end; reproduced as a rejection
/// with a local message.
#[test]
fn rejects_an_importance_annotation_that_is_not_last() {
  for value in [
    "red !important blue",
    "red !important,b",
    "a b !important c d",
    "red !important !important",
    "a !important b !important",
  ] {
    rejects(value, "color", LINT_IMPORTANT_NOT_LAST);
  }
}

// ── Timings ──

/// Below ten the conversion would spell it longer.
#[test]
fn converts_milliseconds_to_seconds_from_ten_up() {
  check(&[
    ("10ms", "transitionDuration", ".01s"),
    ("100ms", "transitionDuration", ".1s"),
    ("999ms", "transitionDuration", ".999s"),
    ("1000ms", "transitionDuration", "1s"),
    ("1500ms", "transitionDuration", "1.5s"),
    ("10.5ms", "transitionDuration", ".0105s"),
    ("9ms", "transitionDuration", "9ms"),
    ("9.999ms", "transitionDuration", "9.999ms"),
    ("1ms", "transitionDuration", "1ms"),
    ("100ms 200ms", "transitionDuration", ".1s .2s"),
    ("calc(100ms + 1s)", "transitionDuration", "calc(.1s + 1s)"),
  ]);
}

/// A negative duration is never converted; a unit merely starting with `ms` is not `ms`.
#[test]
fn converts_only_an_exact_millisecond_unit_at_or_above_the_threshold() {
  check(&[
    ("-100ms", "transitionDuration", "-100ms"),
    ("-10ms", "transitionDuration", "-10ms"),
    ("10msx", "transitionDuration", "10msx"),
    ("0ms", "transitionDuration", "0s"),
  ]);
}

/// Timings runs before the leading-zero pass, so `100ms` → `0.1s` → `.1s`.
#[test]
fn strips_the_leading_zero_off_a_converted_duration() {
  check(&[("100ms", "transitionDuration", ".1s")]);
}

// ── Zero dimensions ──

/// Angles become `0deg`, durations `0s`, any other unit is dropped.
#[test]
fn canonicalizes_a_zero_written_with_a_unit() {
  check(&[
    ("0", "width", "0"),
    ("0px", "width", "0"),
    ("0em 0rem 0vh", "margin", "0 0 0"),
    ("0deg", "transform", "0deg"),
    ("0grad", "transform", "0deg"),
    ("0turn", "transform", "0deg"),
    ("0rad", "transform", "0deg"),
    ("0ms", "transitionDuration", "0s"),
    ("0s", "transitionDuration", "0s"),
    ("0fr", "gridTemplateColumns", "0fr"),
    ("0%", "width", "0%"),
    ("0zz", "width", "0"),
  ]);
}

/// `0.0` and `-0` are left to the leading-zero normalizer.
#[test]
fn reads_the_zero_off_the_spelling_not_the_quantity() {
  check(&[
    ("0.0px", "width", "0px"),
    ("0.000", "opacity", "0"),
    ("-0px", "width", "0px"),
    ("+0px", "width", "0px"),
    ("-0", "zIndex", "0"),
    ("-0%", "width", "0%"),
    ("0.0%", "width", "0%"),
    ("0e0", "width", "0"),
  ]);
}

/// Current-main #1750; expectation from Babel 2d42d72, not the vendored 0.19.1 oracle.
#[test]
fn keeps_a_unit_on_a_zero_inside_every_function() {
  check(&[
    ("translate(0px, 0em)", "transform", "translate(0px,0em)"),
    ("calc(0px + 1px)", "width", "calc(0px + 1px)"),
    (
      "translate(0px) rotate(0deg)",
      "transform",
      "translate(0px) rotate(0deg)",
    ),
    (
      "translate(0px) translate(0em)",
      "transform",
      "translate(0px) translate(0em)",
    ),
    ("var(--a) 0px", "margin", "var(--a) 0"),
    ("0px var(--a)", "margin", "0 var(--a)"),
  ]);
}

/// The exemption is on the property name, so a vendor-prefixed real property is not exempt.
#[test]
fn exempts_a_custom_property_from_zero_canonicalization() {
  check(&[
    ("0px", "--customProp", "0px"),
    ("0deg", "--customProp", "0deg"),
    ("0px", "--x", "0px"),
    ("0px", "-webkit-transform", "0"),
  ]);
}

// ── Leading zero ──

/// Re-spelled through JavaScript number-to-string rules: `0.50` → `.5`, `1.00` untouched.
#[test]
fn strips_the_zero_in_front_of_a_decimal_point() {
  check(&[
    ("0.5px", "width", ".5px"),
    ("0.5", "opacity", ".5"),
    ("0.50", "opacity", ".5"),
    (".5px", "width", ".5px"),
    ("+.5px", "width", ".5px"),
    ("0.2s", "transitionDuration", ".2s"),
    ("rgba(0, 0, 0, 0.5)", "color", "rgba(0,0,0,.5)"),
    (
      "opacity 0.2s ease-in-out",
      "transition",
      "opacity .2s ease-in-out",
    ),
  ]);
}

/// The negative-number asymmetry is the reference compiler's.
#[test]
fn leaves_a_negative_decimal_and_anything_from_one_up_alone() {
  check(&[
    ("-0.5px", "width", "-0.5px"),
    ("-0.5", "opacity", "-0.5"),
    ("1.0px", "width", "1.0px"),
    ("1.00", "opacity", "1.00"),
    ("50.00%", "width", "50.00%"),
    ("1", "opacity", "1"),
    ("-1", "zIndex", "-1"),
  ]);
}

/// `parse_js_float` skips leading JS whitespace but the scanner keeps U+00A0 in the word, so
/// `\u{a0}0.5px` comes back as `.5` (reference behaviour).
#[test]
fn re_spells_a_number_the_unit_split_cannot_read() {
  check(&[
    ("\u{a0}0.5", "opacity", ".5"),
    ("\u{a0}0.5px", "width", ".5"),
  ]);
}

// ── Numeric spelling: the silent-divergence surface ──

/// Re-spelled numbers follow JavaScript's exponent rules; a one-digit difference changes the
/// class name silently.
#[test]
fn spells_a_re_spelled_number_the_way_javascript_does() {
  check(&[
    ("1e3px", "width", "1e3px"),
    ("1e+3px", "width", "1e+3px"),
    ("0.5e1px", "width", "0.5e1px"),
    ("1e-3px", "width", ".001px"),
    (".5e-7px", "width", "5e-8px"),
    ("0.0000001px", "width", "1e-7px"),
    ("0.000000000000000000000001px", "width", "1e-24px"),
    ("0.9999999999999999px", "width", ".9999999999999999px"),
  ]);
}

/// The number stops before an incomplete exponent; the rest becomes the unit.
#[test]
fn stops_a_number_before_an_incomplete_exponent() {
  check(&[("1e", "width", "1e"), ("1e+", "width", "1e+")]);
}

/// `1e400` parses to infinity, not below one; `Infinity` and `NaN` are words.
#[test]
fn leaves_values_at_and_past_the_edge_of_a_double_alone() {
  let huge = format!("1{}px", "0".repeat(400));

  check(&[
    ("1e400px", "width", "1e400px"),
    ("-1e400px", "width", "-1e400px"),
    ("9007199254740993", "zIndex", "9007199254740993"),
    ("Infinity", "width", "Infinity"),
    ("-Infinity", "width", "-Infinity"),
    ("NaN", "width", "NaN"),
  ]);

  assert_eq!(
    normalize_value(&huge, "width", &default_options()),
    huge,
    "a number past the range of a double is not re-spelled"
  );
}

// ── Quotes ──

/// So `''` and `""` hash alike.
#[test]
fn double_quotes_an_empty_string_and_nothing_else() {
  check(&[
    ("''", "content", "\"\""),
    ("\"\"", "content", "\"\""),
    ("'' ''", "content", "\"\" \"\""),
    ("'' \"\"", "content", "\"\" \"\""),
    ("'a'", "fontFamily", "'a'"),
    ("\"a\"", "fontFamily", "\"a\""),
  ]);
}

// ── Camel-cased values ──

/// The two properties whose value is itself a property name.
#[test]
fn dashifies_a_property_name_used_as_a_value() {
  check(&[
    ("backgroundColor", "transitionProperty", "background-color"),
    (
      "backgroundColor, marginTop",
      "transitionProperty",
      "background-color,margin-top",
    ),
    ("marginTop", "willChange", "margin-top"),
    ("backgroundColor", "willChange", "background-color"),
    ("transform, opacity", "willChange", "transform,opacity"),
  ]);
}

#[test]
fn dashifies_nothing_outside_those_two_properties_top_level() {
  check(&[
    ("marginTop", "transition", "marginTop"),
    ("marginTop", "color", "marginTop"),
    ("calc(marginTop)", "transitionProperty", "calc(marginTop)"),
  ]);
}

/// `--fooBar` and `--foobar` are different properties.
#[test]
fn leaves_a_custom_property_name_cased_as_written() {
  check(&[
    ("--fooBar", "transitionProperty", "--fooBar"),
    ("--fooBar", "willChange", "--fooBar"),
  ]);
}

/// Matches Babel's defects: the Microsoft prefix loses its leading dash, and an all-caps value
/// gains one.
#[test]
fn dashifies_by_the_rule_even_where_the_rule_is_wrong() {
  check(&[
    ("MozTransform", "transitionProperty", "-moz-transform"),
    ("WebkitTransform", "transitionProperty", "-webkit-transform"),
    ("msTransform", "transitionProperty", "ms-transform"),
    ("ABC", "transitionProperty", "-abc"),
    ("aB", "transitionProperty", "a-b"),
    ("a-B", "transitionProperty", "a-b"),
  ]);
}

// ── Font size to rem ──

/// Only with the option on, only for `fontSize`, and only for `px`.
#[test]
fn restates_a_pixel_font_size_in_rem_when_asked() {
  check_with(
    &[
      ("16px", "fontSize", "1rem"),
      ("24px", "fontSize", "1.5rem"),
      ("-16px", "fontSize", "-1rem"),
      ("100px 16px", "fontSize", "6.25rem 1rem"),
      ("calc(16px + 1em)", "fontSize", "calc(1rem + 1em)"),
      ("1em", "fontSize", "1em"),
      ("16PX", "fontSize", "16PX"),
      ("16px", "lineHeight", "16px"),
    ],
    &rem_enabled_options(),
  );

  check(&[("16px", "fontSize", "16px")]);
}

/// Conversion runs after the leading-zero pass, so `8px` is `0.5rem`, as Babel emits.
#[test]
fn leaves_the_leading_zero_on_a_converted_font_size() {
  check_with(
    &[
      ("8px", "fontSize", "0.5rem"),
      ("1px", "fontSize", "0.0625rem"),
    ],
    &rem_enabled_options(),
  );
}

/// The zero has already lost its `px` before the conversion runs.
#[test]
fn converts_nothing_for_a_zero_font_size() {
  check_with(&[("0px", "fontSize", "0")], &rem_enabled_options());
}

// ── Strings, escapes and non-ASCII content ──

/// Resolving `\2014 A` would change the bytes and the class name.
#[test]
fn keeps_an_escape_sequence_as_the_author_spelled_it() {
  check(&[
    (
      "\"\\2014 A\", sans-serif",
      "fontFamily",
      "\"\\2014 A\",sans-serif",
    ),
    (
      "'\\2014 A', sans-serif",
      "fontFamily",
      "'\\2014 A',sans-serif",
    ),
    ("\"\\1F600\"", "fontFamily", "\"\\1F600\""),
    ("My\\ Font", "fontFamily", "My\\ Font"),
    ("\"\\\\\"", "fontFamily", "\"\\\\\""),
  ]);
}

#[test]
fn keeps_an_escaped_quote_inside_its_string() {
  check(&[
    ("\"a\\\"b\"", "fontFamily", "\"a\\\"b\""),
    ("'a\\'b'", "fontFamily", "'a\\'b'"),
    ("\"\\\"\"", "fontFamily", "\"\\\"\""),
    (
      "url(\"a\\\")b.png\")",
      "backgroundImage",
      "url(\"a\\\")b.png\")",
    ),
  ]);
}

/// Quoted or not, inside or outside the BMP, and full-width.
#[test]
fn passes_non_ascii_content_through_untouched() {
  check(&[
    ("\"日本語\"", "fontFamily", "\"日本語\""),
    ("日本語", "fontFamily", "日本語"),
    ("'🎉'", "fontFamily", "'🎉'"),
    ("🎉", "content", "🎉"),
    ("'ümlaut'", "fontFamily", "'ümlaut'"),
    ("'ＦＵＬＬＷＩＤＴＨ'", "fontFamily", "'ＦＵＬＬＷＩＤＴＨ'"),
  ]);
}

/// It neither collapses nor gets trimmed.
#[test]
fn treats_a_non_breaking_space_as_part_of_a_word() {
  check(&[
    ("\u{a0}", "color", "\u{a0}"),
    ("a\u{a0}b", "color", "a\u{a0}b"),
  ]);
}

// ── URLs ──

#[test]
fn leaves_a_url_body_opaque() {
  check(&[
    ("url(a.png)", "backgroundImage", "url(a.png)"),
    ("url(a b.png)", "backgroundImage", "url(a b.png)"),
    ("url(a,b.png)", "backgroundImage", "url(a,b.png)"),
    ("url(a(b).png)", "backgroundImage", "url(a(b).png)"),
    ("url()", "backgroundImage", "url()"),
    ("url('a b.png')", "backgroundImage", "url('a b.png')"),
    (
      "url(data:image/svg+xml;base64,AAA=)",
      "backgroundImage",
      "url(data:image/svg+xml;base64,AAA=)",
    ),
  ]);
}

// ── Comments ──

/// `/*/` finds its terminator inside the opening delimiter; a known scanner non-round-trip.
#[test]
fn keeps_a_comment_and_reproduces_the_one_shape_that_does_not_round_trip() {
  check(&[
    ("/* comment */red", "color", "/* comment */red"),
    ("red /* comment */", "color", "red /* comment */"),
    ("/**/", "color", "/**/"),
    ("a/**/b", "color", "a/**/b"),
    ("red /* unterminated", "color", "red /* unterminated"),
    ("/*/ x */", "color", "/**/ x * / "),
  ]);
}

// ── Malformed input ──

/// The report quotes the rule it came from.
#[test]
fn rejects_an_unclosed_function() {
  for value in [
    "calc(",
    "calc(1px",
    "calc((1px)",
    "translate(1px, calc(2px",
    "url(\"a",
  ] {
    rejects(value, "width", LINT_UNCLOSED_FUNCTION);
  }
}

/// Current Babel core emits the same bare diagnostic; the outer host supplies context.
#[test]
fn unclosed_function_report_matches_babel_for_every_key_kind() {
  let options = default_options();
  for key in ["width", "--x", ":hover"] {
    let message = panic_message(catch_unwind(AssertUnwindSafe(|| {
      normalize_value("calc(1px", key, &options)
    })));
    assert_eq!(
      message.trim_start_matches("[StyleX] "),
      LINT_UNCLOSED_FUNCTION,
      "{key}"
    );
  }
}

/// The scanner invents a closing quote rather than failing, so this is the only guard.
#[test]
fn rejects_an_unclosed_string() {
  for value in ["\"unterminated", "'unterminated", "a \"b", "\""] {
    rejects(value, "fontFamily", LINT_UNCLOSED_STRING);
  }
}

#[test]
fn reports_an_unclosed_function_ahead_of_an_unclosed_string() {
  rejects("calc(\"a", "width", LINT_UNCLOSED_FUNCTION);
}

/// Empty, whitespace-only, and every whitespace character the scanner recognizes.
#[test]
fn rejects_a_value_with_no_tokens() {
  for value in ["", " ", "\t", "\n", "\r", "   \t\n  ", "\u{c}"] {
    rejects(value, "color", LINT_VALUE_HAS_NO_TOKENS);
  }
}

/// The structural guard lives at the compiler entry point, not in this fold, so these normalize
/// to something here.
#[test]
fn normalizes_rather_than_rejects_a_malformed_token_sequence() {
  check(&[
    ("calc(1px))", "width", "calc(1px))"),
    (")", "width", ")"),
    ("}", "width", "}"),
    ("{", "width", "{"),
    (";", "width", ";"),
    ("a; color: red", "width", "a; color : red"),
    ("a } .x { color: red", "width", "a } .x { color : red"),
  ]);
}

// ── Custom property references ──

/// Unprefixed names are rejected locally; see `unprefixed_custom_properties_test`.
#[test]
fn normalizes_a_custom_property_reference_like_any_other_function() {
  check(&[
    ("var(--a)", "color", "var(--a)"),
    ("var(--a, red)", "color", "var(--a,red)"),
    ("var(--a,red)", "color", "var(--a,red)"),
    ("var(--A)", "color", "var(--A)"),
    (
      "var(--a, var(--b, var(--c, red)))",
      "color",
      "var(--a,var(--b,var(--c,red)))",
    ),
    (
      "env(safe-area-inset-top)",
      "paddingTop",
      "env(safe-area-inset-top)",
    ),
  ]);
}

// ── Structural extremes ──

/// Not split into a word followed by a signed number.
#[test]
fn keeps_a_unicode_range_whole() {
  check(&[
    ("U+0025-00FF", "unicodeRange", "U+0025-00FF"),
    ("U+26", "unicodeRange", "U+26"),
    ("u+0-7F", "unicodeRange", "u+0-7F"),
  ]);
}

/// The fold has no limit of its own; this pins the depth the compiler's guard permits.
#[test]
fn normalizes_a_value_nested_as_deeply_as_the_compiler_admits() {
  let depth = MAX_VALUE_NESTING_DEPTH;
  let value = format!("{}1px{}", "calc(".repeat(depth), ")".repeat(depth));

  assert_eq!(normalize_value(&value, "width", &default_options()), value);
}

#[test]
fn normalizes_a_value_made_of_very_many_tokens() {
  let value = vec!["0px"; 5_000].join("   ");
  let expected = vec!["0"; 5_000].join(" ");

  assert_eq!(
    normalize_value(&value, "margin", &default_options()),
    expected
  );
}

#[test]
fn normalizes_a_single_very_long_token() {
  let value = format!("\"{}\"", "a".repeat(100_000));

  assert_eq!(
    normalize_value(&value, "fontFamily", &default_options()),
    value
  );
}

// ── The reported divergences ──

/// The six reported divergences; class-name parity is pinned at the transform.
#[test]
fn spells_the_reported_divergences_the_way_the_reference_compiler_does() {
  check(&[
    (
      "opacity 0.2s ease-in-out",
      "transition",
      "opacity .2s ease-in-out",
    ),
    (
      "calc(-1 * var(--spacing))",
      "margin",
      "calc(-1 * var(--spacing))",
    ),
    ("#ffffff", "color", "#ffffff"),
    (
      "'sidebar content'",
      "gridTemplateAreas",
      "'sidebar content'",
    ),
    (
      "color-mix(in srgb, #ff0000 50%, #0000ff)",
      "color",
      "color-mix(in srgb,#ff0000 50%,#0000ff)",
    ),
    (
      "linear-gradient(to right, #fff 0%, #000 100%)",
      "backgroundImage",
      "linear-gradient(to right,#fff 0%,#000 100%)",
    ),
  ]);
}

// ── The fold as a whole ──

/// Not a reference-compiler property, but true of every case here.
#[test]
fn settles_after_one_pass() {
  let options = default_options();

  for (value, key, expected) in ACCEPTED_SAMPLE {
    let twice = normalize_value(expected, key, &options);

    assert_eq!(
      &twice, expected,
      "normalizing `{key}: {value}` a second time moved it"
    );
  }
}

/// Sample for [`settles_after_one_pass`]; excludes `/*/` and `!important`, which don't round-trip.
const ACCEPTED_SAMPLE: &[Case] = &[
  ("#FfF", "color", "#FfF"),
  ("rgba(0, 0, 0, 0.5)", "color", "rgba(0,0,0,.5)"),
  ("100ms", "transitionDuration", ".1s"),
  ("0px", "width", "0"),
  ("0.50", "opacity", ".5"),
  ("''", "content", "\"\""),
  ("backgroundColor", "transitionProperty", "background-color"),
  ("a  /  b", "color", "a / b"),
  ("var( --a , red )", "color", "var(--a,red)"),
  (
    "translate(0px) translate(0em)",
    "transform",
    "translate(0px) translate(0)",
  ),
  (
    "\"\\2014 A\", sans-serif",
    "fontFamily",
    "\"\\2014 A\",sans-serif",
  ),
  ("url( a.png )", "backgroundImage", "url(a.png)"),
];
