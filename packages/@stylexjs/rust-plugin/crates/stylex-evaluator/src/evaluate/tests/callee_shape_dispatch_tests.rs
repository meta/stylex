//! Which of the three callee shapes a call is read as, and what happens to a
//! call that is none of them.
//!
//! `admit_call` matches the callee and hands the call to the helper for its
//! shape: an applied global, a name, or a method. Only the third carries rules
//! of its own, so the match itself is where a call can be sent to the wrong set
//! of rules -- and a call sent to the wrong helper does not fail loudly. It
//! folds, to whatever the other shape's rules answer.
//!
//! So each shape is pinned by an answer only its own helper produces, and the
//! shapes that match none of them are pinned as refusals. The distinctions are
//! deliberately awkward ones: a name in parentheses is still a name, a
//! parenthesised member is still a method, and a computed key that reads as a
//! literal is still not one.

use super::source_evaluation::*;

// ==================== the applied globals ====================

/// The conversions are folded by being called, and each answers something no
/// other shape would: `Array(3)` is a length rather than an element, which is
/// the arm that separates it from every other global.
#[test]
fn an_applied_global_is_answered_by_the_language() {
  assert_folds_to_string("String(1)", "1");
  assert_folds_to_number("Number('2')", 2.0);
  assert_folds_to_number("Array(3).length", 3.0);
  assert_folds_to_number("Object.keys(Object({ a: 1 })).length", 1.0);
}

/// Parentheses are unwrapped before the callee is matched, so a wrapped global
/// reaches the same arm rather than falling to the member arm below.
#[test]
fn parentheses_around_an_applied_global_do_not_change_which_arm_reads_it() {
  assert_folds_to_string("(String)(1)", "1");
  assert_folds_to_string("((String))(1)", "1");
}

// The third arm, a call reached through a name, resolves a module binding --
// which a lone expression has none of, so it cannot be reached from here. It is
// pinned where a module is in scope, in
// `transform_stylex_create_test/refusals_a_module_reaches.rs` and
// `globals_written_as_a_value.rs`.

// ==================== a method call ====================

/// A method has a receiver, and the receiver's own rules are what separates this
/// arm from the two above it.
///
/// That the locale-sensitive names refuse is `engine_fold_tests.rs`'s claim and
/// is pinned there. What is this file's is that the refusal belongs to the
/// *method* arm: the same spelling in the two positions the other arms read --
/// applied as a name, and as a name the module bound -- is not a method, and
/// answers by those arms' rules rather than by this one's.
#[test]
fn a_method_call_reads_the_rules_its_receiver_carries() {
  assert_folds_to_string("'AB'.toLowerCase()", "ab");
  assert_deopts("'AB'.toLocaleLowerCase()");

  // `String` is read by the applied-global arm, which has no receiver and so no
  // locale rule to reach -- the same source text, answered somewhere else.
  assert_folds_to_string("String('AB').toLowerCase()", "ab");
}

/// Parentheses are unwrapped for all three arms, so a wrapped method folds to
/// what the same call folds to bare. Measured against
/// `@stylexjs/babel-plugin@0.19.0`, which folds both spellings alike.
///
/// The receiver inside keeps its own parentheses, and that is not the same
/// rule: the fold prints this source back for the engine to parse, and
/// `{ a: 1 }.valueOf()` opens a block there rather than naming an object. So
/// the callee is read through its brackets and the receiver is carried with
/// them.
#[test]
fn parentheses_around_a_method_reach_the_arm_that_reads_receivers() {
  assert_folds_to_string("'AB'.toLowerCase()", "ab");

  for wrapped in ["('AB'.toLowerCase)()", "(('AB'.toLowerCase))()"] {
    assert_folds_to_string(wrapped, "ab");
  }

  // A receiver that needs its brackets to print, reached through a callee that
  // has none and through one that has.
  assert_folds_to_string("({ a: 'b' }).a", "b");
  assert_folds_to_string("({}).toString()", "[object Object]");
  assert_folds_to_string("(({}).toString)()", "[object Object]");
}

/// A chain hides its middle links, so each link is matched for its own shape
/// rather than the outermost one standing in for all of them.
///
/// That a refusing link refuses the chain is pinned in `engine_fold_tests.rs`.
/// The claim here is the dispatch's: a chain mixes the three arms freely, and
/// every link still reaches the arm its own spelling names.
#[test]
fn every_link_of_a_chain_is_matched_for_its_own_shape() {
  assert_folds_to_string("String(['a', 'b']).split(',').join('-')", "a-b");
  assert_folds_to_number("Object.keys({ a: 1, b: 2 }).join('').length", 2.0);

  // A refusing link buried mid-chain is still what the whole chain answers for,
  // so no link is skipped once an earlier one has folded.
  assert_deopts("String(['a']).toLocaleUpperCase().trim()");
}

// ==================== the shapes that are none of the three ====================

/// A computed key written as a string literal names the method as plainly as a
/// dot does, and the call answers what the language answers: `node` and the
/// reference implementation both fold `'AB'['toLowerCase']()` to `"ab"` (the
/// latter reads a string-literal property off the evaluated receiver,
/// `lib/index.js:6906-6921`; executed).
///
/// A key that is itself an expression is a lookup the reference implementation
/// does not resolve -- it leaves `func` undefined and refuses with
/// `Unsupported expression: CallExpression` (`lib/index.js:6942`; executed) --
/// so that spelling still refuses here.
#[test]
fn a_literal_computed_method_name_folds_and_an_expression_key_refuses() {
  assert_folds_to_string("'AB'['toLowerCase']()", "ab");
  assert_deopts("'AB'[['toLowerCase'][0]]()");
}

/// An arrow written and applied in place is a callee the evaluator resolves to
/// the author's own function, so the call answers what the language answers.
///
/// The reference implementation leaves `func` undefined for a callee that is
/// neither an identifier nor a member and refuses; folding the applied arrow is
/// a deliberate improvement on that, and `'a'` is `node`'s own answer.
#[test]
fn an_arrow_applied_in_place_answers_its_body() {
  assert_folds_to_string("(() => 'a')()", "a");
}

/// A callee that is neither a global, a name, a member, nor an arrow this
/// evaluator resolved reaches no arm at all, and a call that reaches no arm is
/// handed back rather than folded.
///
/// A global's *name* is among them: `String` written as a value resolves to
/// nothing, here as upstream, so a call through one is not a call this module
/// reads.
#[test]
fn a_callee_of_no_shape_reaches_no_arm() {
  for source in [
    "(function () { return 'a'; })()",
    "(0, String)(1)",
    "(true ? String : Number)(1)",
  ] {
    assert_deopts(source);
  }
}

// ==================== the extremes ====================

/// A chain long past anything an author writes still runs the rules once per
/// link rather than losing them, and answers the same value at every length.
///
/// Under a raised ceiling, because a chain this long is deeper than the shipped
/// default admits -- which is the depth ceiling answering rather than the
/// dispatch, and is a different claim, pinned in
/// `transform_stylex_create_test/evaluation_depth_budget.rs`.
#[test]
fn a_very_long_chain_reads_every_link_without_losing_a_rule() {
  let ceiling = 1_024;

  let folding = format!("'ab'{}", ".trim()".repeat(64));
  assert_folds_to_string_with_ceiling(&folding, "ab", ceiling);

  // The one refusing link is buried under sixty-four that fold, and is still
  // what the whole chain answers for -- so no link is skipped for depth. The
  // refusal is this compiler's locale rule (the reference implementation folds
  // a locale-dependent `"AB"`; executed); its method-naming sentence was the
  // optional engine's, so the refusal is asserted without it.
  let refusing = format!("'ab'.toLocaleUpperCase(){}", ".trim()".repeat(64));
  assert_deopts_with_ceiling(&refusing, ceiling);
}

/// The same chain past the shipped ceiling answers for its depth rather than
/// folding, so the two limits are not confused for one another.
#[test]
fn a_chain_deeper_than_the_default_ceiling_is_refused_for_its_depth() {
  let deep = format!("'ab'{}", ".trim()".repeat(64));

  assert_deopt_reason_contains(&deep, "too deeply nested");
}

/// Parentheses nest without bound in the source, and unwrapping them is what
/// decides the arm -- so a deeply wrapped callee must still reach the arm its
/// innermost spelling names.
#[test]
fn deeply_nested_parentheses_still_resolve_to_one_shape() {
  let depth = 32;
  let wrapped = format!("{}String{}(1)", "(".repeat(depth), ")".repeat(depth));

  assert_folds_to_string(&wrapped, "1");
}

// ==================== the phase a rule answers in ====================

/// The three rules that read only the source text answer before anything is
/// resolved, so a receiver the walk could never look up still refuses for its
/// method's own spelling.
///
/// This is the ordering the split makes visible rather than a new behaviour, and
/// it is worth a case of its own because getting it wrong is silent: run after
/// resolution, these would report an unresolved binding for a call whose real
/// fault is the method, and an author would go looking at the wrong half of the
/// line.
///
/// Which sentence answers is reporting, not the guard: the reference
/// implementation evaluates the receiver under a state of its own and reports
/// the outer call (`Unsupported expression: CallExpression`; executed), while
/// this compiler shares one state and reports the receiver's own refusal. Both
/// refuse; the method-naming sentence was the optional engine's. The third row
/// is the locale rule itself: the reference implementation folds a
/// locale-dependent `"2A"` (executed), and this compiler refuses it.
#[test]
fn a_rule_that_reads_only_the_text_answers_before_a_name_is_resolved() {
  // Nothing here binds `unknownName`, so resolution has nothing to answer with.
  assert_deopts("unknownName.toLocaleUpperCase()");
  assert_deopts("unknownName.toLocaleLowerCase()");

  // A receiver that is an expression rather than a name reaches the same rule.
  assert_deopts("(1 + 1 + 'a').toLocaleUpperCase()");
}

/// A number written into the source is a receiver like any other: it answers
/// the methods the language gives a number, and refuses a method it has none
/// of. The receiver's shape no longer decides the call before the method is
/// read, so the rule that used to refuse every written number is stated here as
/// what replaced it.
#[test]
fn a_receiver_written_as_a_number_answers_its_own_methods() {
  assert_folds_to_string("(5).toString()", "5");
  assert_folds_to_string("(5.5).toFixed(2)", "5.50");

  // A method a number does not carry is still not answered -- the dispatch
  // reads the method, so a string's method on a number reaches nothing.
  assert_deopts("(0).toLowerCase()");

  // The same value reached through anything but a written number is not this
  // rule's business, so the phase cannot be refusing on the answer.
  assert_folds_to_string("String(5).padStart(2, '0')", "05");
}
