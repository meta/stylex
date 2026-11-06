//! What a fold hands back when it declines, read at the evaluator's own seam.
//!
//! The transform-level file for this is
//! `tests/transform_stylex_create_test/refusals_that_fall_through.rs`, which
//! measures whole declarations against the reference compiler. What is asserted
//! here is the property that file cannot see directly: a decline is a *deopt
//! with a reason* rather than an abort, and this is which reason.
//!
//! Only the receiver half is readable here. Applying an author's own arrow needs
//! a name the module bound, and this seam evaluates one expression with no module
//! around it — an arrow applied in place is not a call through a name.

use super::source_evaluation::*;

// ──────────────────────────────────────────────
// Candidacy before the rules that read a value
// ──────────────────────────────────────────────

/// A receiver nothing resolves is not the fold's to price, so the reason is the
/// resolution's and not a ceiling. The three spellings agree, which is what a
/// rule running before candidacy had broken for exactly one of them.
#[test]
fn an_unresolved_receiver_answers_the_resolution_rather_than_a_ceiling() {
  for source in [
    "nope.repeat(3)",
    "nope.padStart(4, '0')",
    "nope.padEnd(4, '0')",
    "nope.trim()",
  ] {
    assert_deopt_reason_contains(source, "Referenced constant is not defined.");
  }
}

/// The ceiling still fires on a receiver the fold claimed: the rule moved behind
/// candidacy, it did not go away.
#[test]
fn a_claimed_receiver_past_the_ceiling_still_names_the_rule() {
  assert_deopt_reason_contains(
    "'x'.repeat(200000000)",
    "Cannot bound the string 'repeat' would build.",
  );
}

/// And a claimed receiver inside the ceiling still folds.
#[test]
fn a_claimed_receiver_inside_the_ceiling_still_folds() {
  assert_folds_to_string("'ab'.repeat(3)", "ababab");
  assert_folds_to_string("'7'.padStart(3, '0')", "007");
}

/// The two shapes the optional engine refused on syntax alone.
///
/// A locale-dependent method over a name nothing binds still refuses; which
/// sentence answers is reporting rather than the rule -- the reference
/// implementation evaluates the receiver under a state of its own and reports
/// the outer call (`Unsupported expression: CallExpression`; executed), while
/// this compiler reports the receiver's own refusal.
///
/// A method on a written number is an approved native extension: `node`
/// answers `"1.5"` for `(1.5).toFixed(1)`. The reference implementation calls
/// `Number.prototype.toFixed` with no receiver bound and throws a `TypeError`
/// (`lib/index.js:6896-6904`; executed), so the engine-era refusal named a
/// limitation of neither.
#[test]
fn the_engines_syntax_only_shapes_answer_as_the_language_does() {
  assert_deopts("nope.toLocaleUpperCase()");
  assert_folds_to_string("(1.5).toFixed(1)", "1.5");
}
