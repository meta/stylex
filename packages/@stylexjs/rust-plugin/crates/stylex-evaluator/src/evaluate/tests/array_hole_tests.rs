//! A written array hole, and everything next to it that must not move.
//!
//! `[, 1]` writes two slots and holds one element. The evaluator used to drop
//! the hole, so the array folded one element short of what the source says --
//! `height: [, '2px']` emitted `height: 2px`, and a `length` read through a
//! binding answered one. Neither errored. So the array now refuses, with the
//! reference implementation's own words for a hole: it evaluates element paths
//! and a hole's path carries no node.
//!
//! The suite is two-sided, because refusing every array would satisfy the first
//! half alone. Each refusal is paired with the fold beside it that has to keep
//! working: a trailing comma is punctuation rather than a slot. Source-literal
//! holes refuse at every use, including length, matching Babel and the PR.

use super::source_evaluation::*;
use stylex_constants::constants::evaluation_errors::{PATH_WITHOUT_NODE, SPREAD_ELEMENT};

// ==================== the hole refuses ====================

#[test]
fn a_written_hole_refuses_the_array() {
  for source in ["[, 1]", "[1, , 2]", "[,]", "[, , ,]", "[1, 2, ,]"] {
    assert_hole_refusal(source);
  }
}

/// A hole is refused wherever the array is, so an array nested inside another
/// carries its refusal up rather than folding to a shorter sub-array.
#[test]
fn a_hole_nested_in_another_array_refuses_the_outer_one() {
  for source in ["[[, 1]]", "[1, [2, , 3]]", "[[[[, 1]]]]"] {
    assert_hole_refusal(source);
  }
}

/// A hole in an operand refuses the operand and nothing more. The whole point
/// of a deopt over an abort: an author can write one, and a build that stops on
/// it never reaches the value beside it.
#[test]
fn a_hole_in_an_operand_refuses_rather_than_aborting() {
  for source in [
    "[, 1] || \"a\"",
    "1 ? [, 1] : \"a\"",
    "[...[, 1]]",
    "({ ...[, 1] })",
    "[, 1].join(\"-\")",
  ] {
    assert_deopts(source);
  }
}

/// Depth is not a special case, and a hole a hundred arrays down still refuses
/// for being a hole rather than exhausting anything on the way.
///
/// Runs under a raised ceiling because a hundred arrays is past the shipped
/// default: at the default this input refuses for its depth instead, which is a
/// correct answer to a different question. Which refusal wins at which depth is
/// `tests/transform_stylex_create_test/evaluation_depth_budget.rs`.
#[test]
fn a_hole_a_hundred_arrays_deep_refuses() {
  let mut source = String::from("[, 1]");

  for _ in 0..100 {
    source = format!("[{}]", source);
  }

  assert_hole_refusal_under_ceiling(&source, 512);
}

/// Ten thousand holes is the same answer as one, reached without walking any
/// further than the first.
#[test]
fn ten_thousand_holes_refuse_at_the_first_one() {
  let source = format!("[{}1]", ",".repeat(10_000));

  assert_hole_refusal(&source);
}

// ==================== what a hole is not ====================

/// A trailing comma is punctuation, not a slot, so the array it ends still
/// folds. This is the pair the refusal above is measured against: the parser
/// records `[1, 2,]` as two elements and `[1, 2, ,]` as three, one of them a
/// hole.
#[test]
fn a_trailing_comma_is_not_a_hole() {
  assert_folds_to_slots("[1, 2,]", 2);
  assert_folds_to_slots("[1,]", 1);
  assert_folds_to_slots("[]", 0);
  assert_folds_to_slots("[\"a\", \"b\", \"c\",]", 3);
}

/// The first offending element determines the refusal, as in unchanged Babel.
#[test]
fn a_hole_refuses_an_array_whatever_spread_sits_beside_it() {
  assert_hole_refusal("[, ...\"ab\"]");
  assert_hole_refusal("[...\"ab\", ,]");
  assert_hole_refusal("[...[1], ,]");
}

/// And the operand a spread still cannot resolve is still refused as a spread,
/// which is what keeps the sentence above a statement about the hole.
#[test]
fn a_spread_the_evaluator_cannot_resolve_still_refuses_as_a_spread() {
  assert_deopt_reason_is("[...1]", SPREAD_ELEMENT);
  assert_deopt_reason_is("[...{ a: 1 }]", SPREAD_ELEMENT);
}

// ============ a hole after a spread the evaluator does resolve ============

/// **What a literal hole is in the language, written down so the refusals
/// below are measured against it.**
///
/// A hole is not `undefined`. The two part company at four of the six reads
/// `node` answers for `[..."ab", ,]`, and a compiler that modelled a hole as
/// `undefined` would be wrong at each of them:
///
/// | read | `node` | an `undefined` in the slot |
/// |---|---|---|
/// | `.length` | `3` | `3` -- agrees |
/// | `2 in it` | `false` | `true` |
/// | `Object.keys` | `["0", "1"]` | `["0", "1", "2"]` |
/// | `.map` callback runs | twice | three times |
/// | `[2]` | `undefined` | `undefined` -- agrees |
/// | `.join("-")` | `"a-b-"` | `"a-b-"` -- agrees |
///
/// The two reads that agree are what make this dangerous: an index read and a
/// join both answer correctly for the wrong model, so a compiler can look
/// right at the places most often tested and be wrong at `in`, at the own
/// keys, and at how many times a callback ran.
///
/// This evaluator does not model a literal hole. The constructed-array carrier
/// does, which is a different shape reached a different way. So every read
/// below must **fail closed** -- refuse, and answer no value -- rather than
/// fold whichever of the six happens to be easy. The cases after this one
/// assert exactly that, and they are negative guards: each fails if a partial
/// or wrong value is ever accepted in place of the refusal.
#[test]
fn a_hole_after_a_resolved_spread_is_refused_rather_than_modelled() {
  // Approved spread evaluation can reach the source hole Babel does not reach.
  assert_hole_refusal("[...\"ab\", ,]");

  // And every read off it. Asserted as refusals rather than by sentence: more
  // than one rule can answer first here -- the hole's, and the spread's where
  // the count is read off the source -- and which one gets there is not what
  // these rows are about. What they are about is that none of them answers a
  // value.
  for source in [
    "[...\"ab\", ,].length",
    "[...\"ab\", ,][0]",
    "[...\"ab\", ,][2]",
    "[...\"ab\", ,].join(\"-\")",
    "[...\"ab\", ,].map((each) => each).join(\"-\")",
    "Object.keys([...\"ab\", ,]).join(\"-\")",
  ] {
    assert_deopts(source);
  }
}

/// The control the refusals above are measured against: the same spread with
/// no hole in it folds, and folds to what the language iterates.
///
/// Without this row the case above would pass on an evaluator that refused
/// every spread, which is the behaviour the extension replaced. `node`:
/// `[..."ab"]` is `["a", "b"]`, length two, own keys `["0", "1"]`.
#[test]
fn the_same_spread_without_a_hole_folds_to_what_the_language_iterates() {
  assert_folds_to_strings("[...\"ab\"]", &["a", "b"]);
  assert_folds_to_number("[...\"ab\"].length", 2.0);
  assert_folds_to_string("[...\"ab\"].join(\"-\")", "a-b");
}

// ==================== source-literal length refusals ====================

/// Babel and the PR refuse a source hole before any length can be read.
#[test]
fn a_length_written_on_a_holey_literal_refuses() {
  for source in [
    "[, 1].length",
    "[1, , 2].length",
    "[,].length",
    "[, , ,].length",
    "[1, 2, ,].length",
  ] {
    assert_hole_refusal(source);
  }
}

/// Property spelling and parentheses do not bypass source-hole evaluation.
#[test]
fn a_holey_literals_length_refuses_however_it_is_spelled() {
  for source in [
    "[, 1][\"length\"]",
    "([, 1]).length",
    "(([, 1])).length",
    "([, 1])[\"length\"]",
    "[, 1][`length`]",
  ] {
    assert_hole_refusal(source);
  }
}

/// A key that is not the property, however close it reads. Each of these is a
/// property no array carries, so the receiver is read -- and the receiver
/// refuses for its hole.
#[test]
fn a_key_that_only_looks_like_the_property_reports_the_hole() {
  for source in [
    "[, 1][\"Length\"]",
    "[, 1][\"length \"]",
    "[, 1].lengthx",
    "[, 1][`len${\"x\"}gth`]",
  ] {
    assert_hole_refusal(source);
  }
}

/// Arithmetic cannot bypass a source-hole refusal.
#[test]
fn a_holey_literals_length_refuses_inside_arithmetic() {
  assert_hole_refusal("[, 1].length + 1");
  assert_hole_refusal("[, 1].length * [1, , 2].length");
}

/// A spread is not countable. Keep the first offending element's diagnostic:
/// Babel has no node for a hole, but names a spread reached before the hole.
#[test]
fn a_holey_literal_carrying_a_spread_refuses_the_count() {
  assert_deopt_reason_is("[, ...[1, 2]].length", &format!("{PATH_WITHOUT_NODE}\n"));
  assert_deopt_reason_is("[...[1, 2], , 3].length", SPREAD_ELEMENT);
}

/// Only `length`. Every other property of a holey literal is read off the
/// array, which refuses -- so the refusal an author sees is the hole's, not a
/// second one about the property.
#[test]
fn every_other_property_of_a_holey_literal_reports_the_hole() {
  for source in ["[, 1][0]", "[, 1][1]", "[, 1].foo", "[, 1][\"0\"]"] {
    assert_hole_refusal(source);
  }
}

/// Two source-hole lengths still refuse; no partial count is manufactured.
#[test]
fn two_lengths_off_holey_literals_refuse() {
  assert_hole_refusal("[, 1].length + [, 1].length");
}

// ==================== helpers ====================

/// Asserts the source refuses with the reference implementation's own words for
/// a hole. Exact, because agreeing on the sentence is what makes the diagnostic
/// portable between the two compilers.
#[track_caller]
fn assert_hole_refusal(source: &str) {
  assert_deopt_reason_is(source, &format!("{PATH_WITHOUT_NODE}\n"));
}

/// The same, for a source deep enough to need the ceiling raised first.
#[track_caller]
fn assert_hole_refusal_under_ceiling(source: &str, max_evaluation_depth: usize) {
  assert_deopt_reason_is_in(
    &evaluate_source_with_ceiling(source, max_evaluation_depth),
    source,
    &format!("{PATH_WITHOUT_NODE}\n"),
  );
}

#[track_caller]
fn assert_deopt_reason_is(source: &str, expected: &str) {
  assert_deopt_reason_is_in(&evaluate_source(source), source, expected);
}

#[track_caller]
fn assert_deopt_reason_is_in(
  result: &crate::evaluate_result::EvaluateResult,
  source: &str,
  expected: &str,
) {
  assert!(
    !result.confident,
    "expected `{}` to refuse to fold, got {:?}",
    source, result.value
  );

  assert_eq!(
    result.reason.as_deref(),
    Some(expected),
    "wrong deopt reason for `{}`",
    source
  );
}

/// Asserts the source folds to an array of exactly this many elements. Spelled
/// as a count because a fold that is one element short is what this file is
/// about, and "it folded" passes through that.
#[track_caller]
fn assert_folds_to_slots(source: &str, expected: usize) {
  assert_eq!(
    folds_to_a_list(source).len(),
    expected,
    "wrong element count for `{}`",
    source
  );
}
