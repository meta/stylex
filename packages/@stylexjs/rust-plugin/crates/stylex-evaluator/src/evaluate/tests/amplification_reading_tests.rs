//! What the amplification guard reads a bound off, one shape of receiver at a
//! time.
//!
//! [`amplified_call_tests`](super::amplified_call_tests) asserts that the
//! arithmetic admits the calls a stylesheet is written with. This suite is
//! about the reading *under* that arithmetic: the guard has to see a count, a
//! width and a magnitude before it can multiply anything, and each of the
//! shapes below is a different way of seeing one.
//!
//! **The static pre-measurement this suite was written for is gone.** The guard
//! read a bound off an expression *before* any source crossed into the engine,
//! and refused what it could not bound; the native path has runtime budgets
//! instead, so a call either folds under them or is refused by whatever reads
//! it. The readings below are therefore asserted as what they now are -- the
//! value `node` gives, or the exact refusal both compilers raise -- with every
//! input row kept, because each is still a distinct shape an author writes and
//! a shape that changes answer has to fail here.
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

/// The refusal a call the evaluator reads nothing foldable out of now carries.
///
/// It replaces the two pre-measurement sentences this suite was written with --
/// `Cannot bound the string 'repeat' would build.` and its in-a-callback
/// variant -- which named a bound computed before the call ran. Nothing
/// computes one now. Where the reference implementation reaches the same input
/// it reports this same sentence from its own terminal arm
/// (`utils/evaluate-path.js:1201`), built from a character-identical template.
const UNSUPPORTED_CALL: &str = "Unsupported expression: CallExpression";

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
/// iterates nothing, so the mapper never runs and the answer is the empty
/// array.
///
/// `node`: `Array.from(({ a: 1 }).a, () => 'ab'.repeat(3))` is `[]` -- `1` has
/// no iterator and no `length`, so `Array.from` reads a length of zero. The
/// suite refused this when a bound had to be read for the mapper ahead of time;
/// there is no mapper call to bound.
#[test]
fn a_source_with_no_count_iterates_nothing() {
  assert_folds_to_strings("Array.from(({ a: 1 }).a, () => 'ab'.repeat(3))", &[]);
}

/// `Array.from` with nothing to iterate reads no mapper and declares no length,
/// so the guard admits the call and the language's own complaint is what the
/// author reads.
#[test]
fn array_from_with_nothing_to_iterate_refuses() {
  // `node`: TypeError, "undefined is not iterable". The reference
  // implementation reaches the same call and crashes on it. Refusing is this
  // compiler's answer and stays one; the sentence is the terminal label both
  // write, where the old one quoted the engine's own throw text.
  assert_deopt_reason_contains("Array.from()", UNSUPPORTED_CALL);
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

/// A receiver that resolves to something other than a string carries no
/// `repeat`, so the call refuses.
///
/// `node`: TypeError, "…a.repeat is not a function". The reference
/// implementation gives this exact sentence (its `func` stays undefined and it
/// falls to the terminal arm), so the two agree here character for character.
#[test]
fn a_receiver_that_is_not_a_string_has_no_repeat() {
  assert_deopt_reason_contains("({ a: 1 }).a.repeat(3)", UNSUPPORTED_CALL);
}

/// A count written out below zero asks for nothing, so the guard admits the
/// call and the language's own complaint is what the author reads. The count
/// the guard *rejects* for being negative is one it has to do arithmetic with,
/// which is the case below.
#[test]
fn a_count_written_below_zero_is_left_to_the_language() {
  // `node`: RangeError, "Invalid count value: -1"; the reference
  // implementation calls `String.prototype.repeat` unbounded and crashes on
  // the same throw. Refusing is this compiler's answer. The old sentence
  // quoted the engine's own wording of the throw, which no engine raises now.
  assert_deopt_reason_contains("'ab'.repeat(-1)", UNSUPPORTED_CALL);
}

/// Inside a callback the count is arithmetic over a bound the receiver settled,
/// and a leaf below zero stops that arithmetic -- so no count is read at all and
/// the refusal is the count's own rather than the callback's. A sum and a
/// product are monotone
/// only over values that are not negative, so `(-5) * (-5)` would otherwise read
/// as twenty-five against a bound of nothing.
#[test]
fn a_negative_leaf_folds_per_element_rather_than_stopping_the_call() {
  // `node`: `["", "ab"]` -- the counts are `0` and `1`, neither negative, so
  // nothing throws. The pre-measurement refused this because it could not
  // prove the *sum* stayed non-negative without running it; the elements run
  // now, and each count is what it is.
  assert_folds_to_strings(
    "[1, 2].map((each) => 'ab'.repeat(each + ({ k: -1 }).k))",
    &["", "ab"],
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
#[should_panic(expected = "Unsupported expression: CallExpression")]
fn an_element_below_zero_refuses_the_count_it_sizes() {
  // The safe first row still folds: Node gives "-1!-2!".
  assert_folds_to_string("[-1, 2].map((each) => each + '!').join('-')", "-1!-2!");
  // Node/Babel throw RangeError "Invalid count value: -2" on the first element.
  // PR #2008 at 6fc3a507 and native both throw this plain compiler Error without
  // a frame. Parent-approved transport port: preserve that refusal and text,
  // rather than introduce an independent error-kind channel for this input.
  evaluate_source("[-1, 2].map((each) => 'x'.repeat(each * 2))");
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
  // Both rows refuse for the same reason now -- the literal neither compiler
  // folds -- where the old pair asserted two different sentences because the
  // two shapes were refused in two different places. `node` folds both to
  // `["1!"]`; the reference implementation refuses both as `BigIntLiteral`,
  // having no arm for one, and so does this compiler.
  assert_deopt_reason_contains(
    "Array.from([1n], (each) => each + '!')",
    &unsupported_expression("BigIntLiteral"),
  );
}

/// A count that resolves to a value with no expression form has no number to
/// read, so the call it sizes refuses. A function is the value that reaches it:
/// the evaluator answers one as its own callback rather than as syntax.
#[test]
fn a_count_that_resolves_to_a_function_has_no_number() {
  // `node` folds `""` -- `ToNumber(fn)` is `NaN`, and `repeat(NaN)` is zero
  // repetitions. Refusing is the conservative answer and is kept deliberately:
  // a count nothing can read is not a count this compiler invents a zero for.
  // Only the sentence moves off the pre-measurement's wording.
  assert_deopt_reason_contains("'x'.repeat((() => 2))", UNSUPPORTED_CALL);
}

/// A count leaf a callback writes that neither the scope nor the module can
/// answer for stops the arithmetic, whichever side of the operation it is on.
#[test]
#[should_panic(expected = "Unsupported expression: BigIntLiteral")]
fn a_count_leaf_nothing_can_answer_for_stops_the_arithmetic() {
  // Node throws a mixed-BigInt TypeError. Babel refuses the literal before
  // arithmetic and throws this exact compiler error from the executed callback.
  // Preserve that invocation transport; this is not a returned deopt.
  evaluate_source("[1, 2].map((each) => 'x'.repeat(each + 1n))");
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
fn a_name_the_callback_binds_to_the_whole_receiver_counts_as_the_language_reads_it() {
  // `node`: `[""]` -- the third parameter is the receiver array, `ToNumber`
  // of `["a"]` is `NaN`, and `repeat(NaN)` is zero repetitions. The
  // pre-measurement refused because the receiver was neither an element nor
  // an index and so bounded nothing; the language reads it without a bound.
  assert_folds_to_strings("['a'].map((each, index, all) => 'y'.repeat(all))", &[""]);
}

/// A name nothing at all answers for bounds nothing either, which is the other
/// half of the same reading: one name is bound and unmeasured, the other is not
/// bound anywhere.
#[test]
#[should_panic(expected = "Referenced constant is not defined.")]
fn a_name_nothing_answers_for_bounds_nothing() {
  // Node throws ReferenceError. Babel throws this exact unresolved-name error
  // from the executed callback (utils/evaluate-path.js:511), so match the same
  // invocation transport without changing the source, message or refusal.
  evaluate_source("['a'].map((each) => 'x'.repeat(missing))");
}

/// A length written inside a callback the call did not measure is refused, even
/// where the length itself is a written number.
///
/// A parameter with a default may be handed something else entirely, so the
/// call measures neither the width nor the count for it — and one bounded
/// length repeated an unknown number of times is not bounded at all.
#[test]
fn a_defaulted_parameter_refuses_the_callback_before_any_length_is_read() {
  // A defaulted parameter is an `AssignmentPattern`, not an `Identifier`, so
  // neither compiler folds the arrow: the reference requires
  // `identParams.length === params.length` (`utils/evaluate-path.js:499`) and
  // this one requires every parameter to be a plain name. `node` folds
  // `[["a", "a", "a"]]`; what refuses here is the parameter, one rule ahead of
  // anything about a length.
  assert_deopt_reason_contains(
    "['a'].map((each = 1) => Array(3).fill(each))",
    &unsupported_expression("ArrowFunctionExpression"),
  );
}
