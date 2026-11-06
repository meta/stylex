//! What the amplification guard reads a bound off, one shape of receiver at a
//! time.
//!
//! [`amplified_call_tests`](super::amplified_call_tests) asserts that the
//! arithmetic admits the calls a stylesheet is written with. This suite is
//! about the reading *under* that arithmetic: the guard has to see a count, a
//! width and a magnitude before it can multiply anything, and each of the
//! shapes below is a different way of seeing one.
//!
//! Two of those shapes cannot be written down directly, which is why they have
//! a suite of their own:
//!
//! - **An array the evaluator hands back as the literal it was written as.** An
//!   array written straight down folds to the evaluator's own list, so the arm
//!   that reads a written `ArrayLit` is reached only through a value that
//!   carries one -- `({ a: [1, 2] }).a`, where the member read answers the
//!   property's own node.
//! - **A value a name holds.** A receiver and a count are each resolved through
//!   the module when they are not written in place, so a case about that
//!   resolution has to bind the name.

use super::source_evaluation::*;
use stylex_constants::constants::evaluation_errors::unsupported_expression;

/// The first line every unbounded string refusal shares. The limit follows it
/// and is the project's own option, so the line the author reads is what a case
/// pins.
const UNBOUNDED_STRING: &str = "Cannot bound the string 'repeat' would build.";

/// The same question failed inside a callback, which is a different sentence:
/// the bound that was read there bounds one evaluation rather than the call, so
/// the advice is about the receiver rather than about the count.
const UNBOUNDED_IN_A_CALLBACK: &str =
  "Cannot bound the string 'repeat' would build inside a callback.";

/// A written array is counted by its length and measured by its widest element,
/// exactly as the evaluator's own list is -- so a callback over one folds.
#[test]
fn a_written_array_receiver_is_measured_by_its_elements() {
  assert_folds_to_string(
    "({ a: [1, 2] }).a.map((each) => each + 'px').join('-')",
    "1px-2px",
  );
  assert_folds_to_string(
    "({ a: ['aa', 'b'] }).a.map((each) => each.repeat(2)).join('-')",
    "aaaa-bb",
  );
}

/// The written values with a fixed width -- a boolean, `null`, `undefined` --
/// and a nested array, whose width is its own elements joined. Read off the
/// written element rather than off a resolved value, which is the half of the
/// measurement a list-shaped receiver never exercises.
#[test]
fn a_written_element_of_fixed_width_is_measured_as_it_is_written() {
  assert_folds_to_string(
    "({ a: [true, false] }).a.map((each) => each + '!').join('-')",
    "true!-false!",
  );
  assert_folds_to_string(
    "({ a: [null, undefined] }).a.map((each) => each + '!').join('-')",
    "null!-undefined!",
  );
  assert_folds_to_string(
    "({ a: [[1], [2]] }).a.map((each) => each + '!').join('-')",
    "1!-2!",
  );
}

/// `Array.from` measures its first argument the way a method measures its
/// receiver, so a written array reaches the same reading one argument along.
#[test]
fn array_from_measures_a_written_array_the_same_way() {
  assert_folds_to_string(
    "Array.from(({ a: [1, 2] }).a, (each) => each + 'px').join('-')",
    "1px-2px",
  );
}

/// A string is iterated by code point, so its own length is the count and each
/// element is at most a surrogate pair wide. The one source `Array.from`
/// measures that is not an array at all.
#[test]
fn a_string_source_is_counted_by_its_code_units() {
  assert_folds_to_string("Array.from('ab', (each) => each + '!').join('-')", "a!-b!");
}

/// A source that is neither an array, a string, nor a length-declaring object
/// leaves the mapper unmeasured -- so a body that amplifies inside it refuses
/// rather than folding on a count nothing bounded.
///
/// The refusal names the callback rather than the count, because the count was
/// read: what could not be read is how many times the body runs.
#[test]
fn a_source_with_no_count_leaves_the_mapper_unmeasured() {
  assert_deopt_reason_contains(
    "Array.from(({ a: 1 }).a, () => 'ab'.repeat(3))",
    UNBOUNDED_IN_A_CALLBACK,
  );
}

/// `Array.from` with nothing to iterate reads no mapper and declares no length,
/// so the guard admits the call and the language's own complaint is what the
/// author reads.
#[test]
fn array_from_with_nothing_to_iterate_refuses() {
  assert_deopt_reason_contains(
    "Array.from()",
    "TypeError: cannot convert 'null' or 'undefined' to object",
  );
}

/// A count is arithmetic rather than syntax, so a product bounds a call as well
/// as a literal does -- and the product is what a sum alone would not cover.
#[test]
fn a_count_written_as_a_product_is_worked_out() {
  assert_folds_to_string("'ab'.repeat(2 * 3)", "abababababab");
  assert_folds_to_string("'ab'.repeat((1 + 1) * 2)", "abababab");
}

/// A count a name holds is resolved through the module, so it bounds the call
/// the number itself would bound.
#[test]
fn a_count_a_name_holds_is_worked_out() {
  assert_eq!(
    folded_in_a_module_binding("count", "3", "'ab'.repeat(count)"),
    "ababab"
  );
}

/// A count a name holds as a *list* is read as the array it stands for, which
/// the language turns into a number the same way it turns any other value into
/// one: through the text the array joins to.
///
/// The evaluator carries an array as its own list rather than as a node, so this
/// is the one reading the count takes through the array spelling. All three
/// lengths are the language's own arithmetic: one element is that element, no
/// element is the empty text and so zero, and two elements join to text that is
/// no number at all, which counts as zero rather than as no count.
#[test]
fn a_count_a_name_holds_as_a_list_is_read_as_the_array_it_stands_for() {
  assert_eq!(
    folded_in_a_module_binding("count", "[3]", "'ab'.repeat(count)"),
    "ababab"
  );

  for empty in ["[]", "[1, 2]"] {
    assert_eq!(
      folded_in_a_module_binding("count", empty, "'ab'.repeat(count)"),
      ""
    );
  }
}

/// A receiver a name holds is measured like the string it was given the name
/// of.
#[test]
fn a_receiver_a_name_holds_is_measured_like_the_literal() {
  assert_eq!(
    folded_in_a_module_binding("part", "'ab'", "part.repeat(2)"),
    "abab"
  );
}

/// A receiver that resolves to something other than a string has no length to
/// read, so the call it sizes is unbounded and refuses.
#[test]
fn a_receiver_that_is_not_a_string_has_no_length() {
  assert_deopt_reason_contains("({ a: 1 }).a.repeat(3)", UNBOUNDED_STRING);
}

/// A count written out below zero asks for nothing, so the guard admits the
/// call and the language's own complaint is what the author reads. The count
/// the guard *rejects* for being negative is one it has to do arithmetic with,
/// which is the case below.
#[test]
fn a_count_written_below_zero_is_left_to_the_language() {
  assert_deopt_reason_contains(
    "'ab'.repeat(-1)",
    "RangeError: String.prototype.repeat: count must be non-negative",
  );
}

/// Inside a callback the count is arithmetic over a bound the receiver settled,
/// and a leaf below zero stops that arithmetic -- so no count is read at all and
/// the refusal is the count's own rather than the callback's. A sum and a
/// product are monotone
/// only over values that are not negative, so `(-5) * (-5)` would otherwise read
/// as twenty-five against a bound of nothing.
#[test]
fn a_negative_leaf_stops_the_arithmetic_a_count_is_built_by() {
  assert_deopt_reason_contains(
    "[1, 2].map((each) => 'ab'.repeat(each + ({ k: -1 }).k))",
    UNBOUNDED_STRING,
  );
}

/// The two operations a count is built from, over the two leaves a callback can
/// offer: the element's own magnitude and a number written beside it.
#[test]
fn a_count_a_callback_builds_is_worked_out_from_its_element() {
  assert_folds_to_string(
    "[1, 2].map((each) => 'x'.repeat(each * 2)).join('-')",
    "xx-xxxx",
  );
  assert_folds_to_string(
    "[1, 2].map((each) => 'x'.repeat(each + 1)).join('-')",
    "xx-xxx",
  );
}

/// A leaf the module can answer for is a leaf like a written number, so a count
/// built half from the element and half from the module is still bounded.
#[test]
fn a_count_a_callback_builds_reads_a_leaf_the_module_holds() {
  assert_folds_to_string(
    "[1, 2].map((each) => 'x'.repeat(each + ({ k: 1 }).k)).join('-')",
    "xx-xxx",
  );
}

/// An element below zero is no magnitude, so a receiver holding one measures
/// nothing a count could be built from -- and a body that amplifies over it
/// refuses.
#[test]
fn an_element_below_zero_is_no_magnitude() {
  assert_folds_to_string("[-1, 2].map((each) => each + '!').join('-')", "-1!-2!");
  assert_deopt_reason_contains(
    "[-1, 2].map((each) => 'x'.repeat(each * 2))",
    UNBOUNDED_STRING,
  );
}

/// The last `length` an array-like declares is the one it ends up with, so the
/// reading walks the properties backwards -- and a key that is not `length` is
/// stepped over rather than stopping the walk.
#[test]
fn a_length_declared_before_another_key_is_still_read() {
  assert_folds_to_number("Array.from({ length: 2, a: 1 }).length", 2.0);
  assert_folds_to_number("Array.from({ length: 1, length: 2 }).length", 2.0);
}

/// `{ 'length': n }` declares what `{ length: n }` declares, because the one
/// spelling this reads is what both arrive as.
#[test]
fn the_quoted_spelling_of_the_length_key_is_the_same_key() {
  assert_folds_to_number("Array.from({ 'length': 3 }).length", 3.0);
}

/// A key that is not `length` declares nothing, whatever it holds, so
/// `Array.from` of such an object is the empty array.
///
/// Read against a count past the entry ceiling, which is what makes the pair
/// tell the two readings apart: a key wrongly read as `length` would declare
/// two million elements and be refused, where the same count under a key that
/// is not `length` declares nothing and folds. A smaller count would fold
/// either way and pin nothing.
#[test]
fn a_key_that_is_not_length_declares_nothing() {
  assert_folds_to_number("Array.from({ len: 2000000 }).length", 0.0);
  assert_folds_to_number("Array.from({ 0: 2000000 }).length", 0.0);

  assert_deopt_reason_contains(
    "Array.from({ length: 2000000 })",
    "It declares a length of 2000000 elements",
  );
}

/// A declared length the language will not accept declares nothing this guard
/// has to bound: the language answers the empty array for it, and so does the
/// fold.
#[test]
fn a_length_the_language_rejects_declares_nothing() {
  assert_folds_to_number("Array.from({ length: -1 }).length", 0.0);
}

/// A receiver the evaluator cannot answer for is measured as nothing, so the
/// callback over it carries no count and a body that amplifies refuses. Both
/// readings meet it -- a method's receiver, and `Array.from`'s source, which is
/// the same reading one argument along.
///
/// The two refusals are different sentences because they are refused in
/// different places: a method call falls through to the evaluator's own dispatch
/// and is named by the element it could not read, where `Array.from` is a global
/// the fold owns and refuses under its own name.
#[test]
fn a_receiver_the_evaluator_refuses_is_measured_as_nothing() {
  assert_deopt_reason_contains(
    "[1n].map((each) => each + '!')",
    &unsupported_expression("BigIntLiteral"),
  );
  assert_deopt_reason_contains(
    "Array.from([1n], (each) => each + '!')",
    "Its receiver or one of its arguments is not in a form the compiler can evaluate.",
  );
}

/// A count that resolves to a value with no expression form has no number to
/// read, so the call it sizes refuses. A function is the value that reaches it:
/// the evaluator answers one as its own callback rather than as syntax.
#[test]
fn a_count_that_resolves_to_a_function_has_no_number() {
  assert_deopt_reason_contains("'x'.repeat((() => 2))", UNBOUNDED_STRING);
}

/// A count leaf a callback writes that neither the scope nor the module can
/// answer for stops the arithmetic, whichever side of the operation it is on.
#[test]
fn a_count_leaf_nothing_can_answer_for_stops_the_arithmetic() {
  assert_deopt_reason_contains(
    "[1, 2].map((each) => 'x'.repeat(each + 1n))",
    UNBOUNDED_STRING,
  );
}

/// A written element's magnitude bounds a count the callback builds from it, so
/// a repeat sized by the element folds where the same repeat over an unmeasured
/// receiver refuses. This is the written half of the magnitude reading; the list
/// half is `an_element_below_zero_is_no_magnitude`.
#[test]
fn a_written_elements_magnitude_bounds_the_count_it_sizes() {
  assert_folds_to_string(
    "({ a: [2, 3] }).a.map((each) => 'x'.repeat(each)).join('-')",
    "xx-xxx",
  );
}

/// A nested element with no rendered width leaves that element's whole join
/// unread, so the receiver measures no width. The call still folds, because a
/// repeat of a written count needs no width from the receiver -- what the case
/// pins is that reading an unreadable element does not refuse the call around
/// it.
#[test]
fn one_nested_element_with_no_width_leaves_its_join_unread() {
  assert_folds_to_string(
    "({ a: [[{}], [1]] }).a.map((each) => 'x'.repeat(2)).join('-')",
    "xx-xx",
  );
}

/// A name a callback binds but nothing measured bounds nothing, and the count
/// is refused rather than read off the module.
///
/// The array a callback is handed as its third parameter is such a name: the
/// call measured the receiver's elements and the receiver's indexes, and the
/// receiver itself is neither. Reading the module for it would answer whatever
/// the same spelling holds outside the callback, which is a different value.
#[test]
fn a_name_the_callback_binds_to_nothing_measured_bounds_nothing() {
  assert_deopt_reason_contains(
    "['a'].map((each, index, all) => 'y'.repeat(all))",
    UNBOUNDED_STRING,
  );
}

/// A name nothing at all answers for bounds nothing either, which is the other
/// half of the same reading: one name is bound and unmeasured, the other is not
/// bound anywhere.
#[test]
fn a_name_nothing_answers_for_bounds_nothing() {
  assert_deopt_reason_contains("['a'].map((each) => 'x'.repeat(missing))", UNBOUNDED_STRING);
}

/// A length written inside a callback the call did not measure is refused, even
/// where the length itself is a written number.
///
/// A parameter with a default may be handed something else entirely, so the
/// call measures neither the width nor the count for it — and one bounded
/// length repeated an unknown number of times is not bounded at all.
#[test]
fn a_declared_length_inside_an_unmeasured_callback_is_refused() {
  assert_deopt_reason_contains(
    "['a'].map((each = 1) => Array(3).fill(each))",
    "Cannot bound the array 'Array' would build inside a callback.",
  );
}
