//! Tests candidacy before allocation rules and fallback dispatch for declined calls.

use super::source_evaluation::*;

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

#[test]
fn a_claimed_receiver_past_the_ceiling_still_names_the_rule() {
  assert_deopt_reason_contains(
    "'x'.repeat(200000000)",
    "Cannot bound the string 'repeat' would build.",
  );
}

#[test]
fn a_claimed_receiver_inside_the_ceiling_still_folds() {
  assert_folds_to_string("'ab'.repeat(3)", "ababab");
  assert_folds_to_string("'7'.padStart(3, '0')", "007");
}

#[test]
fn the_engines_syntax_only_shapes_answer_as_the_language_does() {
  assert_deopts("nope.toLocaleUpperCase()");
  assert_folds_to_string("(1.5).toFixed(1)", "1.5");
}
