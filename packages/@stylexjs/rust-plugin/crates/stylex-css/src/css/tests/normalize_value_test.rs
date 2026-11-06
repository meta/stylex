//! The value normalization pipeline, asserted against the reference compiler's
//! own bytes.
//!
//! **Every expectation in this module was produced by running the case through
//! the reference compiler's normalizers, not by reading this compiler and
//! writing down what it ought to say.** That is the only kind of expectation
//! worth having here: a class name is a hash of this text, so an expectation a
//! human believed rather than observed is just the divergence written down
//! twice.
//!
//! The seam is the whole pipeline — value in, declaration text out. The nine
//! normalizers are deliberately *not* asserted one by one. Their contract is
//! the string the fold produces; pinning them individually would re-create the
//! implementation coupling this effort exists to remove, and would fight the
//! next maintainer who diffs them against a new reference-compiler release.
//!
//! **The entry point is [`normalize_value`], not the compiler's own
//! `normalize_css_property_value`, and that is the point.** The latter adds
//! three guards the reference compiler has no counterpart for — the unclosed
//! comment, the nesting budget, the declaration-terminating token — so entering
//! through it would measure this compiler's additions rather than the
//! normalizers. Those guards are asserted in
//! `value_normalization_parity_test.rs`, which does enter there. Two cases
//! below would be rejected outright on that path, and their being normalized
//! here is the contract this module exists to state.
//!
//! Where the reference compiler does something that reads as a defect —
//! `ABC` gaining a leading dash, an importance annotation mid-value crashing —
//! the defect is asserted, not corrected. Hash parity outranks local
//! correctness at this seam, and each such case says so where it sits.

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

/// One case: the authored value, the property it is declared for, and the
/// declaration text the reference compiler produces.
type Case = (&'static str, &'static str, &'static str);

/// Runs a case table under the compiler's defaults.
fn check(cases: &[Case]) {
  check_with(cases, &default_options());
}

/// Runs a case table under `options`.
fn check_with(cases: &[Case], options: &StyleXStateOptions) {
  for (value, key, expected) in cases {
    let actual = normalize_value(value, key, options);

    assert_eq!(&actual, expected, "normalizing `{key}: {value}`");
  }
}

/// Asserts that `value` is rejected for `key` with a message containing
/// `expected`.
///
/// Asserted on the message rather than on the bare fact of a panic: `is_err()`
/// alone passes on any panic at all, including one from an unrelated bug, so a
/// test written that way keeps passing after the guard it watches is gone.
fn rejects(value: &str, key: &str, expected: &str) {
  let options = default_options();
  let result = catch_unwind(AssertUnwindSafe(|| normalize_value(value, key, &options)));
  let message = panic_message(result);

  assert!(
    message.contains(expected),
    "expected `{key}: {value}` to be rejected with `{expected}`, got: {message}"
  );
}

// ---------------------------------------------------------------------------
// What no normalizer touches
// ---------------------------------------------------------------------------

/// The heart of the parity contract. No normalizer understands hex colours,
/// letter case, quote characters or exponent notation, so none of them can
/// alter those — and a value made only of things nobody has an opinion about
/// comes back byte for byte.
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

/// Letter case is never folded, including in units — which is why the
/// unit-sensitive normalizers below all miss their upper-case spellings.
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

// ---------------------------------------------------------------------------
// Whitespace and separators
// ---------------------------------------------------------------------------

/// Runs of whitespace collapse to one space *in place*. Nothing moves to a
/// different position in the value, which is what distinguishes this from
/// re-serializing through a formatter.
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

/// A comma loses the space on both sides; every other separator gains one on
/// both sides, whether or not the author wrote any.
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

/// Whitespace just inside a function's parentheses is removed outright rather
/// than collapsed.
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

// ---------------------------------------------------------------------------
// The importance annotation
// ---------------------------------------------------------------------------

/// The space before `!important` is removed — the one structural edit the
/// whitespace normalizer makes.
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

/// No preceding space means nothing to remove, and the annotation is matched
/// case-sensitively.
#[test]
fn leaves_an_annotation_with_nothing_to_remove_before_it() {
  check(&[
    ("!important", "color", "!important"),
    ("a,!important", "color", "a,!important"),
    ("red !IMPORTANT", "color", "red !IMPORTANT"),
  ]);
}

/// The index the reference implementation tests belongs to the annotation's own
/// sibling list, but the list it removes from is always the top-level one. An
/// annotation written inside a function therefore removes an unrelated
/// top-level node — here the space between `a` and `b`, which is why they come
/// back joined.
///
/// This is a defect in the reference compiler and it is reproduced, not
/// corrected: correcting
/// it would spell the value differently from the reference compiler and name a
/// different class.
#[test]
fn removes_an_unrelated_node_for_an_annotation_written_inside_a_function() {
  check(&[
    ("a b calc(x !important)", "color", "ab calc(x !important)"),
    ("a  b calc(x !important)", "color", "ab calc(x !important)"),
    ("calc(x !important)", "color", "calc(x !important)"),
    // Index 0 has nothing before it, so there is nothing to remove.
    ("a b calc(!important)", "color", "a b calc(!important)"),
    // The node at that index is a separator rather than a space, so the removal
    // is skipped rather than taking the separator with it.
    ("a, b calc(x !important)", "color", "a,b calc(x !important)"),
    // Two annotations, two attempts: the first finds a space and removes it,
    // the second lands on a word and gives up.
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

/// The overrun applies to a function carrying the annotation just as it does to
/// the annotation itself: what matters is whether anything follows at the top
/// level, not where the annotation was written.
#[test]
fn rejects_an_annotation_inside_a_function_that_is_not_the_last_token() {
  for value in ["a b calc(x !important) d", "calc(x !important) a b"] {
    rejects(value, "color", LINT_IMPORTANT_NOT_LAST);
  }
}

/// Removing a node shortens the list the walk is iterating without shortening
/// the walk, so any iteration still to come reads past the end. Only an
/// annotation in the last position escapes it.
///
/// Reproduced as a rejection with a local message. Nothing depends on the
/// wording of a JavaScript runtime error, but everything depends on the value
/// not compiling to something the reference compiler would refuse.
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

// ---------------------------------------------------------------------------
// Timings
// ---------------------------------------------------------------------------

/// Ten milliseconds or more is restated in seconds. Below ten it is left alone,
/// since the conversion would spell it longer than what it replaced.
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

/// A negative duration is below the threshold and so is never converted, and a
/// unit that merely starts with `ms` is not `ms`.
#[test]
fn converts_only_an_exact_millisecond_unit_at_or_above_the_threshold() {
  check(&[
    ("-100ms", "transitionDuration", "-100ms"),
    ("-10ms", "transitionDuration", "-10ms"),
    ("10msx", "transitionDuration", "10msx"),
    ("0ms", "transitionDuration", "0s"),
  ]);
}

/// The order of the fold is behaviour, not arrangement: timings runs before the
/// leading zero specifically so a converted duration is then stripped.
///
/// `100ms` becomes `0.1s` and then `.1s`. Run the two the other way round and
/// it stays `0.1s` — a different class name, and nothing anywhere reports it.
#[test]
fn strips_the_leading_zero_off_a_converted_duration() {
  check(&[("100ms", "transitionDuration", ".1s")]);
}

// ---------------------------------------------------------------------------
// Zero dimensions
// ---------------------------------------------------------------------------

/// Zero angles collapse to `0deg`, zero durations to `0s`, and a zero with any
/// other unit loses its unit entirely.
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

/// Only a number spelled exactly `0` qualifies. `0.0` and `-0` are the same
/// quantity spelled differently and are left to the leading-zero normalizer,
/// which reaches a different answer.
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

/// Inside a function the unit stays, because dropping it would change what the
/// function computes. The test is a comparison of source offsets against the
/// end of the *first* function seen, so a second function later in the value is
/// outside that window and its zero does lose its unit.
///
/// The asymmetry between the two `translate` calls below is the whole of it.
#[test]
fn keeps_a_unit_on_a_zero_inside_the_first_function_only() {
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
      "translate(0px) translate(0)",
    ),
    ("var(--a) 0px", "margin", "var(--a) 0"),
    ("0px var(--a)", "margin", "0 var(--a)"),
  ]);
}

/// A custom property is exempt outright: its value has no grammar the compiler
/// can reason about, and something reading it back as text may well need the
/// unit. The exemption is on the property name, so a vendor-prefixed real
/// property is not covered by it.
#[test]
fn exempts_a_custom_property_from_zero_canonicalization() {
  check(&[
    ("0px", "--customProp", "0px"),
    ("0deg", "--customProp", "0deg"),
    ("0px", "--x", "0px"),
    ("0px", "-webkit-transform", "0"),
  ]);
}

// ---------------------------------------------------------------------------
// Leading zero
// ---------------------------------------------------------------------------

/// A number below one loses the zero in front of its decimal point, and is
/// re-spelled through JavaScript's number-to-string rules rather than copied —
/// which is why `0.50` comes back `.5` and `1.00` comes back untouched.
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

/// A negative number is not below zero *and* at or above it, so it keeps its
/// zero — the asymmetry is the reference compiler's and is reproduced.
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

/// A word can read back as a number and still split into no dimension, and
/// this normalizer's unit-less branch is that word.
///
/// It selects on `parse_js_float`, which skips leading JavaScript whitespace;
/// `unit` does not, and the value scanner does not treat U+00A0 as a space, so
/// the character reaches the word intact. Re-spelling then drops it — and
/// with it the unit, since the split that would have recovered the unit is the
/// one that just failed: `\u{a0}0.5px` comes back as `.5`, not `.5px`.
///
/// That is the reference compiler's behaviour, ternary and all, and it is the
/// reason this normalizer cannot share the word-and-unit walk the others do:
/// that walk skips the word this one rewrites.
#[test]
fn re_spells_a_number_the_unit_split_cannot_read() {
  check(&[
    ("\u{a0}0.5", "opacity", ".5"),
    ("\u{a0}0.5px", "width", ".5"),
  ]);
}

// ---------------------------------------------------------------------------
// Numeric spelling: the silent-divergence surface
// ---------------------------------------------------------------------------

/// Exponent notation is not something any normalizer understands, so a value
/// that keeps its spelling keeps its exponent — but one that falls below one
/// gets re-spelled, and then JavaScript's rules decide whether the result is
/// written out or kept in exponent form.
///
/// This is the highest silent-divergence risk in the pipeline: a float spelled
/// one digit differently is a different class name and nothing reports it.
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

/// An incomplete exponent is not an exponent: the number stops before it, and
/// what is left becomes the unit.
#[test]
fn stops_a_number_before_an_incomplete_exponent() {
  check(&[("1e", "width", "1e"), ("1e+", "width", "1e+")]);
}

/// Boundary conditions a float can reach: past the largest representable
/// double, past the smallest integer that survives one, and the literals that
/// are not finite numbers at all.
///
/// Every one of these is left alone. `1e400` parses to infinity, which is not
/// below one; `Infinity` and `NaN` are words with no leading number.
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

// ---------------------------------------------------------------------------
// Quotes
// ---------------------------------------------------------------------------

/// An empty string gets a double quote so `''` and `""` hash alike. Every other
/// string keeps the quote character the author typed.
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

// ---------------------------------------------------------------------------
// Camel-cased values
// ---------------------------------------------------------------------------

/// The two properties whose value is itself a property name get that name
/// rewritten into its CSS spelling.
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

/// Only those two properties, and only their top-level word tokens: a name
/// written inside a function keeps its case, and so does one declared for any
/// other property.
#[test]
fn dashifies_nothing_outside_those_two_properties_top_level() {
  check(&[
    ("marginTop", "transition", "marginTop"),
    ("marginTop", "color", "marginTop"),
    ("calc(marginTop)", "transitionProperty", "calc(marginTop)"),
  ]);
}

/// A custom property named in one of these keeps its case — `--fooBar` and
/// `--foobar` are different properties.
#[test]
fn leaves_a_custom_property_name_cased_as_written() {
  check(&[
    ("--fooBar", "transitionProperty", "--fooBar"),
    ("--fooBar", "willChange", "--fooBar"),
  ]);
}

/// A vendor-prefixed name written in camel case dashifies by the same rule as
/// any other, which spells the Microsoft prefix without its leading dash — and
/// gives a value in all capitals one it never had.
///
/// Both are defects in the reference compiler and both are reproduced.
/// Correcting either would
/// name a different class than the reference compiler for the same source.
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

// ---------------------------------------------------------------------------
// Font size to rem
// ---------------------------------------------------------------------------

/// Only with the option on, only for `fontSize`, and only for the `px` unit.
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

/// The conversion is appended after the leading-zero normalizer rather than
/// slotted in before it, so the number it produces keeps a leading zero the
/// same number would have lost anywhere else in the value.
///
/// `8px` is `0.5rem`, not `.5rem`. That is the order of the fold showing
/// through, and it is what the reference compiler emits.
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

/// A zero font size has already lost its unit by the time the conversion runs,
/// so there is no `px` left for it to find.
#[test]
fn converts_nothing_for_a_zero_font_size() {
  check_with(&[("0px", "fontSize", "0")], &rem_enabled_options());
}

// ---------------------------------------------------------------------------
// Strings, escapes and non-ASCII content
// ---------------------------------------------------------------------------

/// An escape sequence inside a string is the author's spelling and stays that
/// way. Resolving `\2014 A` to the character it names produces different bytes
/// and therefore a different class name.
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

/// An escaped quote stays inside its own string rather than ending it, so the
/// declaration cannot carry a stray delimiter out into the rule around it.
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

/// Non-ASCII content passes through whether or not it is quoted, whether it is
/// inside or outside the basic multilingual plane, and whether or not it is
/// full-width.
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

/// A non-breaking space is not whitespace to the scanner, so it is part of the
/// word rather than a separator — it neither collapses nor gets trimmed, and a
/// value made of nothing else is a value with a token in it.
#[test]
fn treats_a_non_breaking_space_as_part_of_a_word() {
  check(&[
    ("\u{a0}", "color", "\u{a0}"),
    ("a\u{a0}b", "color", "a\u{a0}b"),
  ]);
}

// ---------------------------------------------------------------------------
// URLs
// ---------------------------------------------------------------------------

/// A `url()` body is one opaque token, so characters that look like CSS syntax
/// inside it are not read as syntax.
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

// ---------------------------------------------------------------------------
// Comments
// ---------------------------------------------------------------------------

/// A comment survives as itself. The one exception is `/*/`, where the scan
/// finds its terminator inside the opening delimiter; that is a documented
/// non-round-trip of the scanner and correcting it would change class names.
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

// ---------------------------------------------------------------------------
// Malformed input
// ---------------------------------------------------------------------------

/// An unclosed function is rejected, and the report quotes the rule it came
/// from so the author can find the declaration.
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

/// The rejection names the declaration, and a custom property — which has no
/// grammar of its own — is reported under a stand-in property name.
#[test]
fn names_the_declaration_in_an_unclosed_function_report() {
  let options = default_options();

  let message = panic_message(catch_unwind(AssertUnwindSafe(|| {
    normalize_value("calc(1px", "width", &options)
  })));
  assert!(
    message.contains("* { width: calc(1px }"),
    "expected the report to quote the rule, got: {message}"
  );

  let custom = panic_message(catch_unwind(AssertUnwindSafe(|| {
    normalize_value("calc(1px", "--x", &options)
  })));
  assert!(
    custom.contains("* { color: calc(1px }"),
    "expected a custom property to be reported under a stand-in, got: {custom}"
  );

  let pseudo = panic_message(catch_unwind(AssertUnwindSafe(|| {
    normalize_value("calc(1px", ":hover", &options)
  })));
  assert!(
    pseudo.contains(":hover calc(1px"),
    "expected a pseudo selector to be reported rule-shaped, got: {pseudo}"
  );
}

/// An unclosed string is rejected. The scanner invents the closing quote rather
/// than failing, so this is the only thing between an unterminated string and a
/// declaration that swallows whatever followed it.
#[test]
fn rejects_an_unclosed_string() {
  for value in ["\"unterminated", "'unterminated", "a \"b", "\""] {
    rejects(value, "fontFamily", LINT_UNCLOSED_STRING);
  }
}

/// An unclosed function is looked for before an unclosed string, so a value
/// carrying both is reported as the function.
#[test]
fn reports_an_unclosed_function_ahead_of_an_unclosed_string() {
  rejects("calc(\"a", "width", LINT_UNCLOSED_FUNCTION);
}

/// A value that scans to no tokens at all fails, because there is no first node
/// to read. Empty, whitespace-only, and every whitespace character the scanner
/// recognizes.
#[test]
fn rejects_a_value_with_no_tokens() {
  for value in ["", " ", "\t", "\n", "\r", "   \t\n  ", "\u{c}"] {
    rejects(value, "color", LINT_VALUE_HAS_NO_TOKENS);
  }
}

/// Stray delimiters, unbalanced brackets and token sequences that are not valid
/// CSS are not rejected here. The scanner never fails, so they normalize to
/// something — and the guard that refuses a value able to break out of its own
/// rule is a separate, deliberate local addition that sits at the compiler's
/// entry point rather than in this fold.
///
/// Asserted so that the division of labour is written down: if a value like the
/// last two below ever reaches a stylesheet, the missing guard is the defect,
/// not this pipeline.
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

// ---------------------------------------------------------------------------
// Custom property references
// ---------------------------------------------------------------------------

/// A custom property reference normalizes like any other function: the fold
/// has no opinion about its name, only about the whitespace around its
/// arguments.
///
/// A name missing its leading double hyphen is rejected outright, by a local
/// pass with no reference-compiler equivalent — asserted in
/// `unprefixed_custom_properties_test`, not here, because a rejection has no
/// spelling for a case table to compare.
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

// ---------------------------------------------------------------------------
// Structural extremes
// ---------------------------------------------------------------------------

/// A unicode range is recognized as one token, so it is not mistaken for a word
/// followed by a signed number and cut in half.
#[test]
fn keeps_a_unicode_range_whole() {
  check(&[
    ("U+0025-00FF", "unicodeRange", "U+0025-00FF"),
    ("U+26", "unicodeRange", "U+26"),
    ("u+0-7F", "unicodeRange", "u+0-7F"),
  ]);
}

/// Nesting as deep as the compiler admits normalizes without incident.
///
/// The fold recurses once per level and has no limit of its own; the limit is
/// the one the compiler applies before normalization is entered, so this pins
/// the depth that limit permits rather than a number of its own choosing.
#[test]
fn normalizes_a_value_nested_as_deeply_as_the_compiler_admits() {
  let depth = MAX_VALUE_NESTING_DEPTH;
  let value = format!("{}1px{}", "calc(".repeat(depth), ")".repeat(depth));

  assert_eq!(normalize_value(&value, "width", &default_options()), value);
}

/// A value made of very many sibling tokens is a different shape of extreme
/// from a deeply nested one — it costs list length rather than stack — and is
/// normalized in one pass.
#[test]
fn normalizes_a_value_made_of_very_many_tokens() {
  let value = vec!["0px"; 5_000].join("   ");
  let expected = vec!["0"; 5_000].join(" ");

  assert_eq!(
    normalize_value(&value, "margin", &default_options()),
    expected
  );
}

/// A single token of substantial length is passed through without being
/// re-spelled or truncated.
#[test]
fn normalizes_a_single_very_long_token() {
  let value = format!("\"{}\"", "a".repeat(100_000));

  assert_eq!(
    normalize_value(&value, "fontFamily", &default_options()),
    value
  );
}

// ---------------------------------------------------------------------------
// The reported divergences
// ---------------------------------------------------------------------------

/// The six values whose spelling this effort exists to correct, asserted at
/// this seam as the declaration text the reference compiler produces.
///
/// The class name is what the report is really about, and it is pinned
/// separately where the transform can see it — but a class name is a hash of
/// this text, so an agreement here is the precondition for one there.
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

// ---------------------------------------------------------------------------
// The fold as a whole
// ---------------------------------------------------------------------------

/// Normalizing an already-normalized value changes nothing further.
///
/// Not a property the reference compiler states, but one every case in this
/// module happens to have, and a cheap way to catch a normalizer that rewrites
/// on every pass rather than settling.
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

/// A cross-section of the module's cases, reused by [`settles_after_one_pass`].
/// Deliberately excludes the shapes that are documented not to round-trip —
/// `/*/`, and the importance annotation, whose output is no longer a value the
/// scanner reads back the same way.
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
