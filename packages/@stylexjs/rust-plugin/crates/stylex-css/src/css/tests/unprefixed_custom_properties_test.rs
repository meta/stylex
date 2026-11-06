//! The unprefixed-custom-property rejection at the public entry point.
//!
//! No reference-compiler equivalent (it accepts `var(foo)`), so rejections are not harness
//! verdicts; accepted values are spelled identically by both. Only top-level `var()` is checked,
//! the first argument must be a word, and the function name is matched case-sensitively.

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

/// `-foo` is a legal identifier that names no property.
#[test]
fn a_single_leading_dash_is_rejected() {
  rejects_unprefixed("color", &["var(-foo)", "var(-)"]);
}

#[test]
fn surrounding_whitespace_does_not_hide_the_mistake() {
  rejects_unprefixed("color", &["var( foo )", "var(\tfoo\n)", "var(   foo)"]);
}

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

#[test]
fn the_bare_prefix_is_accepted() {
  accepts(&[
    unchanged("color", "var(--)"),
    unchanged("color", "var(----x)"),
  ]);
}

#[test]
fn a_reference_with_no_argument_is_accepted() {
  accepts(&[unchanged("color", "var()")]);
}

/// A limit of the rule; widening it would reject programs that compile today.
#[test]
fn a_nested_reference_is_out_of_reach() {
  accepts(&[
    unchanged("width", "calc(var(foo) + 10px)"),
    same("color", "rgb(var(r), 0, 0)", "rgb(var(r),0,0)"),
    same("color", "var(--a, var(b))", "var(--a,var(b))"),
  ]);
}

/// Function names are case-insensitive in CSS but matched exactly here.
#[test]
fn the_reference_name_is_matched_case_sensitively() {
  accepts(&[
    unchanged("color", "VAR(foo)"),
    unchanged("color", "Var(foo)"),
    unchanged("color", "vAr(foo)"),
  ]);
}

#[test]
fn a_cased_reference_to_a_prefixed_property_is_accepted() {
  accepts(&[unchanged("color", "VAR(--foo)")]);
}

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

#[test]
fn a_dashed_function_call_is_accepted() {
  accepts(&[
    unchanged("color", "--custom-fn(red)"),
    unchanged("color", "--custom-fn(foo)"),
  ]);
}

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

/// A `url()` body is scanned as one opaque word.
#[test]
fn a_reference_spelled_inside_a_url_is_accepted() {
  accepts(&[
    unchanged("backgroundImage", "url(var(foo))"),
    unchanged("backgroundImage", "url(\"var(foo)\")"),
  ]);
}

#[test]
fn a_first_argument_that_is_not_a_word_is_accepted() {
  accepts(&[
    unchanged("color", "var(\"foo\")"),
    unchanged("color", "var('foo')"),
    unchanged("color", "var(/* foo */ bar)"),
    same("color", "var(, foo)", "var(,foo)"),
  ]);
}

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

#[test]
fn a_non_ascii_property_name_is_judged_by_its_prefix() {
  accepts(&[
    unchanged("color", "var(--fóo)"),
    unchanged("color", "var(--日本語)"),
    unchanged("color", "var(--emoji-🎨)"),
  ]);

  rejects_unprefixed("color", &["var(fóo)", "var(日本語)", "var(🎨)"]);
}

/// The scanner does not resolve escapes, so `\--foo` begins with a backslash.
#[test]
fn an_escaped_prefix_does_not_count_as_a_prefix() {
  rejects_unprefixed("color", &["var(\\--foo)", "var(\\2D\\2D foo)"]);
}

#[test]
fn an_unclosed_reference_is_reported_as_unclosed() {
  rejects(
    "color",
    &["var(foo", "var(--foo", "var(foo, red"],
    LINT_UNCLOSED_FUNCTION,
    &default_options(),
  );
}

#[test]
fn an_unclosed_string_beside_a_bad_reference_is_reported_as_unclosed() {
  rejects(
    "color",
    &["var(foo) \"abc", "\"abc var(foo)"],
    LINT_UNCLOSED_STRING,
    &default_options(),
  );
}

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

#[test]
fn a_long_run_of_references_is_walked_in_full() {
  let mut value = "var(--a) ".repeat(256);
  value.push_str("var(z)");

  rejects_unprefixed("color", &[value.as_str()]);
}

/// Built at run time, so not a [`Case`]; the assertion is the one [`check`] makes.
#[test]
fn a_long_run_of_good_references_is_accepted() {
  let value = "var(--a) ".repeat(256);
  let expected = value.trim_end();

  let actual = normalize_css_property_value("color", expected, &default_options());

  assert_eq!(actual, expected, "normalizing `color: {expected}`");
}
