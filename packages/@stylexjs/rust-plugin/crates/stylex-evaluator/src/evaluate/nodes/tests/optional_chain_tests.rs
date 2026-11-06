//! What `?.` answers.
//!
//! The form asks one question before it reads anything: is the base there. A
//! base that is `null` or `undefined` makes the whole chain answer nothing,
//! and every other base is read exactly as the same expression without the
//! question mark would be read. Both halves are pinned here, in both the
//! member and the call spelling, because a chain that stopped early on a base
//! that *is* there would silently drop a declaration the author wrote.
//!
//! **This compiler folds a chain the reference implementation refuses, and
//! keeps doing so.** `@stylexjs/babel-plugin` 0.19.0 has no `Optional` handling
//! at all and answers `Unsupported expression: OptionalMemberExpression` for
//! every base, so `({ color: 'red' })?.color` gives CSS here and stops a build
//! there. Measured again for ticket 50 of `.scratch/split-transform-crate` and
//! decided: this compiler does *more* here rather than something else. The fold
//! is the language's own answer, nothing it writes is a value the source does
//! not describe, and an author who writes `?.` gets CSS here and a build error
//! there. A build that compiles where the other stops is the one direction a
//! parting is safe in.

use crate::evaluate::source_evaluation::*;
use stylex_constants::constants::evaluation_errors::unsupported_expression;

/// A base the evaluator holds as one of its own values -- an array is its own
/// list rather than an expression -- is still a base that is there, so the
/// chain reads through it.
#[test]
fn a_chain_reads_through_a_base_the_evaluator_holds_as_its_own_value() {
  assert_folds_to_number("['a', 'b']?.length", 2.0);
  assert_folds_to_string("['a', 'b']?.[1]", "b");
}

/// The ordinary base: an object written where it is read.
#[test]
fn a_chain_reads_through_an_object_base() {
  assert_folds_to_string("({ color: 'red' })?.color", "red");
  assert_folds_to_string("({ theme: { color: 'red' } })?.theme?.color", "red");
}

/// A method reached through the question mark is a call the evaluator does not
/// fold, in either compiler. The refusal names the optional call rather than
/// the method, which is the shape an author has to change.
#[test]
fn a_method_called_through_the_question_mark_refuses() {
  for source in ["['a', 'b']?.join('-')", "'red'?.toUpperCase()"] {
    assert_deopt_reason_contains(source, &unsupported_expression("OptionalCallExpression"));
  }
}

/// The call spelling of the same question. An optional call is refused as an
/// optional call whatever its callee is, so an arrow written where it is called
/// is not applied: the reference plugin has no arm for it and answers exactly
/// `Unsupported expression: OptionalCallExpression` (oracle run of the
/// unchanged plugin; terminal arm `lib/index.js:6942`). Optional member reads
/// are the supported extension; optional calls are not.
#[test]
fn an_optional_call_refuses_whatever_its_callee() {
  assert_deopt_reason_contains(
    "((color) => color)?.('red')",
    &unsupported_expression("OptionalCallExpression"),
  );
}

/// A nullish base stops the chain, so nothing is read and no value comes back.
/// The declaration is left to the runtime rather than written from a value the
/// source does not describe.
#[test]
fn a_nullish_base_stops_the_chain() {
  for source in [
    "null?.color",
    "undefined?.color",
    "null?.()",
    "undefined?.()",
    "null?.['color']",
  ] {
    assert_deopts(source);
  }
}
