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
//! working: a trailing comma is punctuation rather than a slot, and a `length`
//! written on a literal receiver is still counted from the source -- the one
//! reading that answers what the language says.

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

/// A spread is refused ahead of the operand it spreads, so an array carrying
/// both a spread and a hole reads whichever comes first -- the refusals are
/// ordered by position, not ranked.
#[test]
fn a_spread_and_a_hole_are_refused_in_the_order_they_are_written() {
  assert_deopt_reason_is("[, ...[1]]", PATH_WITHOUT_NODE);
  assert_deopt_reason_is("[...[1], ,]", SPREAD_ELEMENT);
}

// ==================== `length` still counts the slots ====================

/// The count is read off the source, so it survives the refusal of the array
/// itself. `[, 1].length` is two in the language, and answering two is the only
/// reading that is not confidently wrong.
#[test]
fn a_length_written_on_a_holey_literal_counts_the_slots() {
  assert_folds_to_number("[, 1].length", 2.0);
  assert_folds_to_number("[1, , 2].length", 3.0);
  assert_folds_to_number("[,].length", 1.0);
  assert_folds_to_number("[, , ,].length", 3.0);
  assert_folds_to_number("[1, 2, ,].length", 3.0);
}

/// Both spellings of the same property, and a receiver in parentheses, which is
/// not a different receiver.
#[test]
fn a_holey_literals_length_is_counted_however_it_is_spelled() {
  assert_folds_to_number("[, 1][\"length\"]", 2.0);
  assert_folds_to_number("([, 1]).length", 2.0);
  assert_folds_to_number("(([, 1])).length", 2.0);
  assert_folds_to_number("([, 1])[\"length\"]", 2.0);
  assert_folds_to_number("[, 1][`length`]", 2.0);
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

/// A count is a number, so it composes with the arithmetic the evaluator folds.
#[test]
fn a_holey_literals_length_composes_as_a_number() {
  assert_folds_to_number("[, 1].length + 1", 3.0);
  assert_folds_to_number("[, 1].length * [1, , 2].length", 6.0);
}

/// A spread is still not countable, whatever else the array holds. One written
/// element stands for however many the spread holds, so neither the written
/// count nor the evaluated one is the language's answer.
#[test]
fn a_holey_literal_carrying_a_spread_refuses_the_count() {
  assert_deopt_reason_is("[, ...[1, 2]].length", SPREAD_ELEMENT);
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

/// A count read off a holey literal is still a count, so two of them add. A
/// holey array reached through a *binding* has no literal at the read and
/// refuses instead; that shape needs a module and is pinned in
/// `tests/transform_stylex_create_test/array_style_values.rs`.
#[test]
fn two_counts_off_holey_literals_add() {
  assert_folds_to_number("[, 1].length + [, 1].length", 4.0);
}

// ==================== helpers ====================

/// Asserts the source refuses with the reference implementation's own words for
/// a hole. Exact, because agreeing on the sentence is what makes the diagnostic
/// portable between the two compilers.
#[track_caller]
fn assert_hole_refusal(source: &str) {
  assert_deopt_reason_is(source, PATH_WITHOUT_NODE);
}

/// The same, for a source deep enough to need the ceiling raised first.
#[track_caller]
fn assert_hole_refusal_under_ceiling(source: &str, max_evaluation_depth: usize) {
  assert_deopt_reason_is_in(
    &evaluate_source_with_ceiling(source, max_evaluation_depth),
    source,
    PATH_WITHOUT_NODE,
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
