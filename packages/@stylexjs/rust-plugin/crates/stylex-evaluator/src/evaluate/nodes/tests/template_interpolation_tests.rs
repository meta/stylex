//! What a template literal writes for the value it interpolates.
//!
//! Interpolation is `ToString` rather than "is this a literal the evaluator
//! recognises". Every value the language has a string for contributes that
//! string, so `${null}`, `${true}`, `${[1, 2]}` and `${{}}` each write what a
//! browser would write -- and none of them contributes the empty string, which
//! is what declared a value the author never wrote and hashed a class name to
//! match it.
//!
//! The one value with no string at all is a function. It is refused rather than
//! answered: the reference implementation writes the source text of its own
//! evaluator closure there, which is an internal artifact with no business in a
//! stylesheet.

use crate::evaluate::source_evaluation::*;
use stylex_constants::constants::evaluation_errors::TEMPLATE_LITERAL;
use stylex_constants::constants::messages::EXPRESSION_IS_NOT_A_STRING;

/// The values whose string form is not their literal spelling. Each was
/// previously dropped, so each is written out rather than left to a group.
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

/// An escape is written as the character it stands for, not as the two
/// characters the source spells it with.
#[test]
fn an_escape_is_written_as_the_character_it_names() {
  assert_folds_to_string("`a\\tb`", "a\tb");
  assert_folds_to_string("`a\\u0062c`", "abc");
}

/// A template with no interpolation at all is its own text, and one with
/// nothing but an interpolation is that value's string.
#[test]
fn a_template_with_no_interpolation_is_its_own_text() {
  assert_folds_to_string("`red`", "red");
  assert_folds_to_string("``", "");
  assert_folds_to_string("`${'red'}`", "red");
}

/// A function has no compile-time string, so the template refuses rather than
/// writing a value that stands for nothing an author asked for.
#[test]
fn a_function_interpolated_into_a_template_refuses() {
  let source = format!("`a${{{FOLD_FUNCTION}}}b`");

  assert_refused_with(
    &evaluated_against_a_function_fold(&source),
    &source,
    EXPRESSION_IS_NOT_A_STRING,
  );
}

/// The namespace is an object rather than a function, so it writes the object
/// default instead of refusing -- the same parting the coercions make.
#[test]
fn a_namespace_interpolated_into_a_template_writes_the_object_default() {
  let source = format!("`a${{{FOLD_NAMESPACE}}}b`");

  assert_eq!(
    folded_text_of(evaluated_against_a_function_fold(&source), &source),
    "a[object Object]b"
  );
}

/// An interpolation that folds to nothing leaves the template with no text at
/// all. The refusal matters more than the value: a shorter string standing in
/// its place would be a declaration the source does not describe.
#[test]
fn an_interpolation_that_folds_to_nothing_refuses_the_whole_template() {
  assert_deopts("`a${unknownName}b`");
  assert_deopts("`a${'x'.constructor}b`");
}

/// The text a template grows is measured against the character ceiling as it
/// grows, so an interpolation that passes it refuses with the ceiling's own
/// sentence -- which names the template rather than the value inside it,
/// because it is the whole literal that became too large.
#[test]
fn an_interpolation_past_the_ceiling_refuses_the_whole_template() {
  assert_refused_at_the_character_ceiling(
    4,
    &a_function_fold(),
    TEMPLATE_LITERAL,
    &format!("`${{['1234567890']}}${{{FOLD_FUNCTION}}}`"),
  );
}

/// The written text is measured on the same terms as the interpolations, so a
/// template whose own literal passes the ceiling refuses before anything is
/// interpolated into it.
///
/// The two halves of a template grow one buffer, and only one of them was
/// written by the author. Measuring the interpolations alone would let a
/// literal of any size through, which is the declaration this refusal is about.
#[test]
fn a_written_quasi_past_the_ceiling_refuses_the_whole_template() {
  assert_refused_at_the_character_ceiling(
    4,
    &a_function_fold(),
    TEMPLATE_LITERAL,
    "`1234567890${1}`",
  );
}
