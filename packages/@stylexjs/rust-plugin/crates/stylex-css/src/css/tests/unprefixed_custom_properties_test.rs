//! The unprefixed-custom-property rejection, asserted at the public entry
//! point.
//!
//! This rule has **no reference-compiler equivalent**: `var(foo)` is accepted
//! there and emitted verbatim. So unlike every other module beside it, the
//! expectations here are not harness verdicts — there is nothing to compare
//! against for a value the two compilers disagree about *accepting*. What the
//! harness does still say, and what the accepting cases below stand on, is that
//! every value this rule lets through is spelled identically by both. A case
//! that starts failing here is either a rejection that moved or a spelling that
//! moved, and the second kind belongs to the parity modules.
//!
//! The rule reads the token list: a top-level `var()` whose first argument is a
//! word not starting with `--`. Three consequences of that reading are pinned
//! below rather than left to be rediscovered — a nested reference is out of
//! reach, a first argument that is not a word is not a name, and the function
//! name is matched case-sensitively.

use crate::css::{
  common::normalize_css_property_value,
  tests::support::{Case, check, default_options, rejects, same, unchanged},
};

use stylex_constants::constants::messages::{
  LINT_RULE_BREAKING_TOKEN, LINT_UNCLOSED_COMMENT, LINT_UNCLOSED_FUNCTION, LINT_UNCLOSED_STRING,
  LINT_VALUE_NESTED_TOO_DEEPLY, UNPREFIXED_CUSTOM_PROPERTIES,
};

/// Asserts that each value is rejected for naming an unprefixed property.
fn rejects_unprefixed(property: &str, values: &[&str]) {
  rejects(
    property,
    values,
    UNPREFIXED_CUSTOM_PROPERTIES,
    &default_options(),
  );
}

/// Asserts that each case is accepted and spelled as the case says.
fn accepts(cases: &[Case]) {
  check(cases, &default_options());
}

#[test]
fn a_reference_missing_its_prefix_is_rejected() {
  rejects_unprefixed("color", &["var(foo)", "var(x)", "var(someVariableName)"]);
}

#[test]
fn a_reference_missing_its_prefix_is_rejected_whatever_follows_it() {
  rejects_unprefixed(
    "color",
    &["var(foo, red)", "var(foo,red)", "var(foo, var(--bar))"],
  );
}

/// The prefix is two dashes, and one is a different mistake with the same
/// runtime consequence: `-foo` is a legal identifier that names no property.
#[test]
fn a_single_leading_dash_is_rejected() {
  rejects_unprefixed("color", &["var(-foo)", "var(-)"]);
}

/// Whitespace inside the parentheses belongs to the function node, not to the
/// name, so padding a mistake does not hide it.
#[test]
fn surrounding_whitespace_does_not_hide_the_mistake() {
  rejects_unprefixed("color", &["var( foo )", "var(\tfoo\n)", "var(   foo)"]);
}

/// Only one of the references has to be wrong.
#[test]
fn one_bad_reference_among_good_ones_is_enough() {
  rejects_unprefixed(
    "color",
    &[
      "var(--a) var(b)",
      "var(b) var(--a)",
      "var(--a) var(--b) var(c) var(--d)",
    ],
  );
}

/// A word that is not an identifier at all is still not a property name. The
/// rule asks one question — does the name start with `--` — and a number, a hex
/// colour or a dimension answers it the same way a typo does.
#[test]
fn a_first_argument_that_is_a_word_but_not_a_name_is_rejected() {
  rejects_unprefixed("color", &["var(1px)", "var(#fff)", "var(0)", "var(50%)"]);
}

#[test]
fn a_prefixed_reference_is_accepted() {
  accepts(&[
    unchanged("color", "var(--foo)"),
    unchanged("color", "var(--xAbCdEf)"),
    unchanged("backgroundColor", "var(----__hashed_var__1jqb1tb)"),
    unchanged("color", "var(--a) var(--b) var(--c)"),
  ]);
}

#[test]
fn a_prefixed_reference_with_a_fallback_is_accepted() {
  accepts(&[
    same("color", "var(--foo, red)", "var(--foo,red)"),
    same("color", "var(--a, var(--b))", "var(--a,var(--b))"),
    same(
      "padding",
      "var(--rightpadding, 20px)",
      "var(--rightpadding,20px)",
    ),
  ]);
}

/// The bare prefix is a name of nothing, and the rule has no opinion about it:
/// it asks for the prefix and finds it.
#[test]
fn the_bare_prefix_is_accepted() {
  accepts(&[
    unchanged("color", "var(--)"),
    unchanged("color", "var(----x)"),
  ]);
}

/// A reference with no argument names nothing, so there is no name to be
/// missing a prefix.
#[test]
fn a_reference_with_no_argument_is_accepted() {
  accepts(&[unchanged("color", "var()")]);
}

/// The rule walks the top level of the value only. A reference nested inside
/// another function is not reached, whatever it names — pinned because it is a
/// limit of the rule rather than a property of the input, and widening it would
/// start rejecting programs that compile today.
#[test]
fn a_nested_reference_is_out_of_reach() {
  accepts(&[
    unchanged("width", "calc(var(foo) + 10px)"),
    same("color", "rgb(var(r), 0, 0)", "rgb(var(r),0,0)"),
    same("color", "var(--a, var(b))", "var(--a,var(b))"),
  ]);
}

/// Function names are case-insensitive in CSS but matched exactly here, so an
/// unconventionally cased reference is not inspected.
#[test]
fn the_reference_name_is_matched_case_sensitively() {
  accepts(&[
    unchanged("color", "VAR(foo)"),
    unchanged("color", "Var(foo)"),
    unchanged("color", "vAr(foo)"),
  ]);
}

/// A prefixed reference under any casing is accepted for the same reason, so
/// the casing rule never costs an author a working declaration.
#[test]
fn a_cased_reference_to_a_prefixed_property_is_accepted() {
  accepts(&[unchanged("color", "VAR(--foo)")]);
}

/// Some other function that happens to take a bare identifier is not a
/// custom-property reference.
#[test]
fn another_function_taking_a_bare_word_is_accepted() {
  accepts(&[
    unchanged("content", "attr(data-value)"),
    same("width", "min(100%, 500px)", "min(100%,500px)"),
    same("width", "max(100px, 200px)", "max(100px,200px)"),
    same("color", "rgb(255, 0, 0)", "rgb(255,0,0)"),
    unchanged("width", "calc(100% - 20px)"),
  ]);
}

/// A CSS dashed-function call is its own construct and shares nothing with a
/// custom-property reference but the dashes.
#[test]
fn a_dashed_function_call_is_accepted() {
  accepts(&[
    unchanged("color", "--custom-fn(red)"),
    unchanged("color", "--custom-fn(foo)"),
  ]);
}

/// Text that spells a reference inside a string is text.
#[test]
fn a_reference_spelled_inside_a_string_is_accepted() {
  accepts(&[
    unchanged("content", "\"var(foo)\""),
    unchanged("content", "'var(foo)'"),
    same(
      "fontFamily",
      "\"var(foo)\", sans-serif",
      "\"var(foo)\",sans-serif",
    ),
  ]);
}

/// A `url()` body is scanned as one opaque word, so nothing inside it is a
/// function to inspect.
#[test]
fn a_reference_spelled_inside_a_url_is_accepted() {
  accepts(&[
    unchanged("backgroundImage", "url(var(foo))"),
    unchanged("backgroundImage", "url(\"var(foo)\")"),
  ]);
}

/// A first argument the author wrote as something other than a word is not a
/// name spelled wrong.
#[test]
fn a_first_argument_that_is_not_a_word_is_accepted() {
  accepts(&[
    unchanged("color", "var(\"foo\")"),
    unchanged("color", "var('foo')"),
    unchanged("color", "var(/* foo */ bar)"),
    same("color", "var(, foo)", "var(,foo)"),
  ]);
}

/// A value carrying no reference at all never reaches the check's interesting
/// branch, which is most values.
#[test]
fn a_value_with_no_reference_is_accepted() {
  accepts(&[
    unchanged("color", "red"),
    unchanged("margin", "10px"),
    unchanged("padding", "0"),
    unchanged("display", "block"),
    unchanged("content", "\"\""),
  ]);
}

/// Non-ASCII survives the scan, and a non-ASCII name is judged by its prefix
/// like any other.
#[test]
fn a_non_ascii_property_name_is_judged_by_its_prefix() {
  accepts(&[
    unchanged("color", "var(--fóo)"),
    unchanged("color", "var(--日本語)"),
    unchanged("color", "var(--emoji-🎨)"),
  ]);

  rejects_unprefixed("color", &["var(fóo)", "var(日本語)", "var(🎨)"]);
}

/// An escape is characters, not an escape: the scanner does not resolve one, so
/// `\--foo` is a name beginning with a backslash and the prefix is not there.
#[test]
fn an_escaped_prefix_does_not_count_as_a_prefix() {
  rejects_unprefixed("color", &["var(\\--foo)", "var(\\2D\\2D foo)"]);
}

/// Reported ahead of this rule, because a reference the author never finished
/// writing is more usefully described as unfinished than as misnamed. Both
/// spellings below are unprefixed *and* unclosed; the unclosed report wins.
#[test]
fn an_unclosed_reference_is_reported_as_unclosed() {
  rejects(
    "color",
    &["var(foo", "var(--foo", "var(foo, red"],
    LINT_UNCLOSED_FUNCTION,
    &default_options(),
  );
}

/// The unclosed-string detector runs ahead of this rule too, so a value broken
/// in both ways is reported once, by the earlier pass.
#[test]
fn an_unclosed_string_beside_a_bad_reference_is_reported_as_unclosed() {
  rejects(
    "color",
    &["var(foo) \"abc", "\"abc var(foo)"],
    LINT_UNCLOSED_STRING,
    &default_options(),
  );
}

/// The structural guards in front of the pipeline run earlier still: a value
/// that could not be spelled into the stylesheet at all is rejected for that,
/// before any pass sees it.
#[test]
fn a_structurally_impossible_value_is_reported_before_this_rule() {
  let options = default_options();

  rejects(
    "color",
    &["var(foo) } color: red", "var(foo); margin: 10px"],
    LINT_RULE_BREAKING_TOKEN,
    &options,
  );

  rejects(
    "color",
    &["var(foo) /* unterminated"],
    LINT_UNCLOSED_COMMENT,
    &options,
  );
}

/// A value nested past the recursion budget is rejected for its depth, not for
/// what it happens to name at the bottom of it.
#[test]
fn a_value_nested_too_deeply_is_reported_before_this_rule() {
  let depth = crate::css::common::MAX_VALUE_NESTING_DEPTH + 1;
  let value = format!("{}var(foo){}", "calc(".repeat(depth), ")".repeat(depth));

  rejects(
    "color",
    &[value.as_str()],
    LINT_VALUE_NESTED_TOO_DEEPLY,
    &default_options(),
  );
}

/// The rule is about the value, so the property it is declared for makes no
/// difference — including when that property is itself a custom property.
#[test]
fn the_declared_property_does_not_change_the_verdict() {
  rejects_unprefixed("--my-var", &["var(foo)"]);
  rejects_unprefixed("transitionProperty", &["var(foo)"]);
  rejects_unprefixed("gridTemplateAreas", &["var(foo)"]);

  accepts(&[
    unchanged("--my-var", "var(--foo)"),
    unchanged("transitionProperty", "var(--foo)"),
  ]);
}

/// A long run of references is walked in full, so a bad one at the far end is
/// still found: there is nothing about position that lets a later one through.
#[test]
fn a_long_run_of_references_is_walked_in_full() {
  let mut value = "var(--a) ".repeat(256);
  value.push_str("var(z)");

  rejects_unprefixed("color", &[value.as_str()]);
}

/// The same run with nothing wrong in it is accepted, which is the half of the
/// test above that says the rejection came from the one bad reference rather
/// than from the length.
///
/// Built at run time, so it cannot go in a [`Case`] table — those hold
/// `&'static str`. The assertion is the one [`check`] makes, spelled out.
#[test]
fn a_long_run_of_good_references_is_accepted() {
  let value = "var(--a) ".repeat(256);
  let expected = value.trim_end();

  let actual = normalize_css_property_value("color", expected, &default_options());

  assert_eq!(actual, expected, "normalizing `color: {expected}`");
}
