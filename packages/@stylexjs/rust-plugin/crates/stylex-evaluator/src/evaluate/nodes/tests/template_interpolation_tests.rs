//! Tests template interpolation, coercion refusals and character limits.

use crate::evaluate::source_evaluation::*;
use stylex_constants::constants::evaluation_errors::TEMPLATE_LITERAL;
use stylex_constants::constants::messages::EXPRESSION_IS_NOT_A_STRING;

#[test]
fn every_value_with_a_string_contributes_it() {
  assert_folds_to_string("`a${null}b`", "anullb");
  assert_folds_to_string("`a${true}b`", "atrueb");
  assert_folds_to_string("`a${undefined}b`", "aundefinedb");
  assert_folds_to_string("`a${[1, 2]}b`", "a1,2b");
  assert_folds_to_string("`a${({})}b`", "a[object Object]b");
  assert_folds_to_string("`a${0}b`", "a0b");
  assert_folds_to_string("`a${''}b`", "ab");
}

#[test]
fn an_escape_is_written_as_the_character_it_names() {
  assert_folds_to_string("`a\\tb`", "a\tb");
  assert_folds_to_string("`a\\u0062c`", "abc");
}

#[test]
fn a_template_with_no_interpolation_is_its_own_text() {
  assert_folds_to_string("`red`", "red");
  assert_folds_to_string("``", "");
  assert_folds_to_string("`${'red'}`", "red");
}

#[test]
fn a_function_interpolated_into_a_template_refuses() {
  let source = format!("`a${{{FOLD_FUNCTION}}}b`");

  assert_refused_with(
    &evaluated_against_a_function_fold(&source),
    &source,
    EXPRESSION_IS_NOT_A_STRING,
  );
}

/// Unlike Babel's wrapper-source interpolation, functions have no compile-time string here.
#[test]
fn an_arrow_interpolated_into_a_template_refuses() {
  assert_deopt_reason_contains(
    "`a${() => 1}b`",
    "Cannot convert a function to a compile-time primitive.",
  );
}

#[test]
fn a_namespace_interpolated_into_a_template_writes_the_object_default() {
  let source = format!("`a${{{FOLD_NAMESPACE}}}b`");

  assert_eq!(
    folded_text_of(evaluated_against_a_function_fold(&source), &source),
    "a[object Object]b"
  );
}

#[test]
fn an_interpolation_that_folds_to_nothing_refuses_the_whole_template() {
  assert_deopts("`a${unknownName}b`");
  assert_deopts("`a${'x'.constructor}b`");
}

#[test]
fn an_interpolation_past_the_ceiling_refuses_the_whole_template() {
  assert_refused_at_the_character_ceiling(
    4,
    &a_function_fold(),
    TEMPLATE_LITERAL,
    &format!("`${{['1234567890']}}${{{FOLD_FUNCTION}}}`"),
  );
}

#[test]
fn a_written_quasi_past_the_ceiling_refuses_the_whole_template() {
  assert_refused_at_the_character_ceiling(
    4,
    &a_function_fold(),
    TEMPLATE_LITERAL,
    "`1234567890${1}`",
  );
}
