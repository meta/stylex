//! `Object.keys`, `Object.values` and `Object.entries` over a receiver the
//! engine never sees.
//!
//! The three statics fold in the engine on every ordinary receiver, so a call
//! that reaches this compiler's own walk is one whose *receiver* the engine
//! declined. The [folded function
//! map](../../../../../CONTEXT.md#folded-function-map) is the one that matters:
//! it is not a JavaScript value at all, it carries own keys upstream, and
//! answering the empty list for it spread nothing and compiled a style object
//! the author never wrote.
//!
//! The receivers with no `ToObject` at all still have to refuse, and with the
//! sentence the reference implementation gives.
//!
//! **One reading of an array.** The receiver is the value the evaluator
//! resolved, never the array literal beside it, so an array reaches this walk
//! having already been read the way every other reader reads it. A hole and an
//! element that will not fold therefore refuse the declaration here exactly as
//! they refuse it anywhere else, with the same sentence the reference
//! implementation gives.
//!
//! **The one deliberate parting.** A value with no expression form -- a
//! callback, an entry of the function fold -- is a receiver this compiler
//! cannot write down whole, so it refuses. The reference implementation holds
//! the real function and counts its slot. Refusing is the answer because the
//! alternative is a list one element short, which is CSS the source does not
//! describe; and because `keys`, `values` and `entries` are one question asked
//! three ways and must not answer it three ways.

use super::source_evaluation::*;
use crate::evaluate_result::EvaluateResult;
use stylex_constants::constants::evaluation_errors::{
  OBJECT_METHOD, PATH_WITHOUT_NODE, unsupported_expression,
};
use stylex_constants::constants::messages::{ILLEGAL_PROP_ARRAY_VALUE, NULLISH_TO_OBJECT};

/// The three spellings of the same question, so every case asks all three and a
/// walk that answered one of them differently is visible.
const QUESTIONS: [&str; 3] = ["keys", "values", "entries"];

/// The list `Object.<question>(<receiver>)` answers, as its length.
#[track_caller]
fn counted(question: &str, receiver: &str) -> f64 {
  let source = format!("Object.{question}({receiver}).length");

  number_of(evaluated_against_a_function_fold(&source), &source)
}

/// What `source` folds to over a state whose memo already answers nothing for
/// `(() => 1) + 1`.
///
/// A receiver holding a value that resolved to nothing needs two doors open at
/// once. The memo is the one thing that answers nothing while the walk stays
/// confident, and the function fold is what makes the engine decline the call.
fn folded_after_the_memo(source: &str) -> Box<EvaluateResult> {
  evaluated_after_against(UNRESOLVED_MEMO_WARM, &a_function_fold(), source)
}

/// The same as [`assert_refuses`], over that state.
#[track_caller]
fn assert_refuses_after_the_memo(question: &str, receiver: &str, reason: &str) {
  let source = format!("Object.{question}({receiver})");

  assert_refused_with(&folded_after_the_memo(&source), &source, reason);
}

/// The number `result` folded to.
#[track_caller]
fn number_of(result: Box<EvaluateResult>, source: &str) -> f64 {
  match folded_value_of(result, source)
    .as_expr()
    .and_then(|expr| expr.as_lit())
  {
    Some(swc_core::ecma::ast::Lit::Num(number)) => number.value,
    other => panic!("expected `{}` to count, got {:?}", source, other),
  }
}

#[track_caller]
fn assert_refuses(question: &str, receiver: &str, reason: &str) {
  let source = format!("Object.{question}({receiver})");
  let result = evaluated_against_a_function_fold(&source);

  assert!(
    !result.confident,
    "expected `{}` to refuse, got {:?}",
    source, result.value
  );
  assert_eq!(
    result.reason.as_deref(),
    Some(reason),
    "wrong refusal for `{}`",
    source
  );
}

/// The namespace has the keys it was registered with, and one entry of it has
/// the one key the reference implementation wraps it under. Answering the empty
/// list here spread nothing and compiled a style object the author never wrote.
#[test]
fn a_function_fold_has_the_keys_it_was_registered_with() {
  for question in QUESTIONS {
    assert_eq!(counted(question, "sx"), 1.0);
    assert_eq!(counted(question, "own"), 1.0);
  }
}

/// The keys are the names, not their positions, so the namespace answers what
/// it holds rather than an index.
#[test]
fn the_keys_of_a_function_fold_are_its_names() {
  let result = evaluated_against_a_function_fold("Object.keys(sx)[0]");

  assert!(
    result.confident,
    "expected the keys to fold, got a deopt: {:?}",
    result.reason
  );

  match result.value {
    Some(value) => assert_eq!(folded_text(&value), "create"),
    None => panic!("expected the keys to answer a value"),
  }
}

/// An array holding a hole does not fold, in any position, so the receiver
/// refuses and the declaration stops -- which is what the reference
/// implementation does with the same source, under the same sentence.
#[test]
fn an_array_holding_a_hole_refuses() {
  for question in QUESTIONS {
    for receiver in ["[, 'a']", "['a', , 'b']", "[, ,]", "[[, 'a'], 'b']"] {
      assert_refuses(question, receiver, PATH_WITHOUT_NODE);
    }
  }
}

/// The key list a fold answered is itself an array, and its own keys are its
/// indices. It arrives as the literal the fold wrote rather than as the
/// evaluator's own list, which is the second of an array's two spellings -- a
/// reader that knew only the first answered the empty list here, while the same
/// compiler spread the same keys correctly one function away.
#[test]
fn the_key_list_of_a_fold_carries_its_own_indices() {
  for question in QUESTIONS {
    assert_eq!(counted(question, "Object.keys(sx)"), 1.0);
    assert_eq!(counted(question, "Object.entries(sx)"), 1.0);
  }

  let source = "Object.values(Object.keys(sx))[0]";

  assert_eq!(
    folded_text_of(evaluated_against_a_function_fold(source), source),
    "create"
  );
}

/// An element of such an array is read by the rule every array element is read
/// by, at the top level as well as under it. Read only below the top, a value
/// this compiler cannot write down was refused one level down and written into
/// a style value at the top -- two answers for one value.
#[test]
fn an_element_is_read_by_the_same_rule_at_every_depth() {
  for question in QUESTIONS {
    for receiver in ["[own.fn]", "[[own.fn]]", "[[[own.fn]]]"] {
      assert_refuses(
        question,
        &format!("sx.missing ?? {receiver}"),
        ILLEGAL_PROP_ARRAY_VALUE,
      );
    }
  }
}

/// A string's own properties are its indices, one per character.
#[test]
fn a_string_has_one_key_per_character() {
  for question in QUESTIONS {
    assert_eq!(counted(question, "'abc'"), 3.0);
    assert_eq!(counted(question, "''"), 0.0);
  }
}

/// A value with no own enumerable properties answers the empty list rather than
/// refusing. A function is the one that reaches this walk: it is what an entry
/// of the fold stands for, and `Object.assign({}, () => 1)` is `{}` upstream.
#[test]
fn a_value_with_no_own_keys_answers_the_empty_list() {
  for question in QUESTIONS {
    assert_eq!(counted(question, "own.fn"), 0.0);
  }
}

/// `null` and `undefined` have no `ToObject` at all, so the statics throw in
/// the language rather than answering the empty list. Refused with the sentence
/// the reference implementation gives, because folding `[]` there is CSS the
/// source does not describe.
#[test]
fn a_nullish_receiver_refuses() {
  for question in QUESTIONS {
    assert_refuses(question, "sx.missing", NULLISH_TO_OBJECT);
  }
}

// ==================== a receiver reached past a declined fold ====================
//
// A receiver the engine can read is folded there, so a case about this walk has
// to put one behind something the engine declines. `sx.missing ?? <value>` is
// that shape: the fold declines the whole expression for the name it cannot
// carry, and the value behind the `??` is what the walk then reads.

/// The list `Object.<question>` answers for a value reached past a declined
/// fold, as its length.
#[track_caller]
fn counted_past_the_fold(question: &str, receiver: &str) -> f64 {
  counted(question, &format!("sx.missing ?? {receiver}"))
}

/// A string's own properties are its indices, and an array's are its own --
/// both read out of the evaluated value rather than out of the syntax, because
/// a value reached this way has no literal left to read.
#[test]
fn an_evaluated_string_or_array_is_read_from_the_value() {
  for question in QUESTIONS {
    assert_eq!(counted_past_the_fold(question, "'abc'"), 3.0);
    assert_eq!(counted_past_the_fold(question, "['a', 'b']"), 2.0);
    assert_eq!(counted_past_the_fold(question, "[['a'], ['b']]"), 2.0);
    assert_eq!(counted_past_the_fold(question, "[['a', null]]"), 1.0);
  }
}

/// A string is indexed by UTF-16 code unit, which is what the language counts.
///
/// The two readings agree for every string inside the Basic Multilingual Plane
/// and part company on an astral character: it is two code units and one Rust
/// character, so a character count answered one key where the language answers
/// two, and shifted every index after it.
///
/// Each code unit of an astral character is a lone surrogate, which no Rust
/// string holds, so the receiver is refused rather than answered with a key
/// list that is short or a character nobody wrote. A spread of the same string
/// already reads it that way.
///
/// The reference implementation answers `['0', '1']` for the astral character:
/// the keys are the indices whatever the units hold, so a key list *could* be
/// answered without its values. Measured for ticket 50 of
/// `.scratch/split-transform-crate` and decided against, because `keys`,
/// `values` and `entries` are one question asked three ways: the values are two
/// lone surrogates this compiler cannot write, so answering the keys alone
/// would make one spelling fold where the other two refuse. An index read on
/// the same string parts company here on purpose: it answers the replacement
/// character, because `charAt` already does and because the reference
/// implementation's own half becomes that character once it is written to a
/// file -- `array_index_tests.rs::a_string_reads_an_index_by_code_unit`. What
/// a list cannot do is answer two of them and call it the string.
#[test]
fn a_string_receiver_is_read_by_code_unit() {
  for question in QUESTIONS {
    assert_eq!(counted_past_the_fold(question, "''"), 0.0);
    assert_eq!(counted_past_the_fold(question, "'\\u00e9'"), 1.0);

    // The astral character alone, and one with a character after it -- where a
    // character count answered two keys and named the second one `1`, which
    // the language calls `2`. The third is the half of an astral character
    // written on its own, which the text holds before any index is read off it.
    for receiver in ["'\\u{1F600}'", "'\\u{1F600}a'", "'\\ud83d'"] {
      let source = format!("Object.{question}(sx.missing ?? {receiver}).length");

      assert_refused_with(
        &evaluated_against_a_function_fold(&source),
        &source,
        ILLEGAL_PROP_ARRAY_VALUE,
      );
    }
  }
}

/// `null` written as an element is a value rather than an absence, so it keeps
/// its own key -- unlike the hole above it, which occupies a slot and carries
/// none.
#[test]
fn a_written_null_element_keeps_its_key() {
  for question in QUESTIONS {
    assert_eq!(counted_past_the_fold(question, "[null, 'a']"), 2.0);
  }
}

/// A value that is not an object has no own enumerable properties, so it
/// answers the empty list. `Object.keys(1)` is `[]` in the language and in the
/// reference implementation.
#[test]
fn a_value_that_is_not_an_object_answers_the_empty_list() {
  for question in QUESTIONS {
    assert_eq!(counted_past_the_fold(question, "1"), 0.0);
  }
}

/// An array holding an element the walk cannot write down refuses the whole
/// receiver, at every depth. Answering the empty list dropped the array, and
/// answering a list without that element dropped one value out of it -- both
/// are CSS the source does not describe.
///
/// This is the deliberate parting the module doc names: the reference
/// implementation holds the real function and counts its slot.
#[test]
fn an_element_with_no_expression_form_refuses_the_receiver() {
  for question in QUESTIONS {
    for receiver in ["[own]", "[[own]]", "[[[own]]]", "[() => 1]", "[[() => 1]]"] {
      assert_refuses(
        question,
        &format!("sx.missing ?? {receiver}"),
        ILLEGAL_PROP_ARRAY_VALUE,
      );
    }
  }
}

/// A property carried by a getter has no value the walk can read, so the whole
/// receiver refuses rather than answering a list with the property missing.
///
/// The sentence comes from the object walk rather than from this one: an object
/// literal is evaluated before the receiver is read, and a getter is refused
/// there. So an [evaluator-written
/// object](../../../CONTEXT.md#evaluator-written-object) is all this reader can
/// be handed, and every property of one is a key and a value.
#[test]
fn a_property_that_is_not_a_value_refuses_the_receiver() {
  for question in QUESTIONS {
    assert_refuses(
      question,
      "sx.missing ?? ({ get a() { return 1; } })",
      OBJECT_METHOD,
    );
  }
}

// ==================== an array written where it is read ====================

/// An array literal is evaluated like every other argument, so what the
/// receiver reader is handed is the value it folded to. A nested array of
/// readable elements is written down as a nested array, which is the one key
/// the outer array carries.
#[test]
fn a_nested_array_literal_is_one_readable_key() {
  for question in QUESTIONS {
    assert_eq!(counted(question, "[['a']]"), 1.0);
  }
}

/// An array literal holding an element with no compile-time form refuses, and
/// the refusal says what a style array may hold.
#[test]
fn an_array_literal_with_an_unreadable_element_refuses() {
  for question in QUESTIONS {
    assert_refuses(question, "[own, 'a']", ILLEGAL_PROP_ARRAY_VALUE);
    assert_refuses(question, "[[own]]", ILLEGAL_PROP_ARRAY_VALUE);
  }
}

/// An element the dispatch folds in no position refuses the declaration where
/// it is written, before the receiver is read at all. A regular expression is
/// such an element, and the reference implementation refuses the same source
/// under the same sentence.
#[test]
fn an_element_that_refuses_to_fold_refuses_the_declaration() {
  let regexp = unsupported_expression("RegExpLiteral");

  for question in QUESTIONS {
    assert_refuses(question, "[/re/]", &regexp);
    assert_refuses(question, "[/re/, 'a']", &regexp);
  }
}

/// A callback has no object form of its own, so a receiver that *is* one reads
/// as a value with no own keys -- the same answer a number gets, and the one
/// the reference implementation gives. An array *holding* one is the separate
/// question above, because there the slot has to be written down.
#[test]
fn a_callback_receiver_answers_the_empty_list() {
  for question in QUESTIONS {
    assert_eq!(counted_past_the_fold(question, "(() => 1)"), 0.0);
  }
}

/// A three-level array is written down whole, which is the one shape that reads
/// a nested array out of another nested array. The count and the value at the
/// bottom are both the reference implementation's answers for the same array.
#[test]
fn a_three_level_array_is_one_key_holding_its_own_levels() {
  for question in QUESTIONS {
    assert_eq!(counted_past_the_fold(question, "[[['a']]]"), 1.0);
  }

  // The levels rather than the count, because a walk that flattened them would
  // answer the same one key.
  let source = "Object.values(sx.missing ?? [[['a']]])[0][0][0]";

  assert_eq!(
    folded_text_of(evaluated_against_a_function_fold(source), source),
    "a"
  );
}

// ==================== a value that resolved to nothing ====================
//
// The third value class a receiver can hold, beside a value and an absence: an
// element the walk read confidently and found nothing for. Only the memo
// answers that way, and only a declined call reaches this walk -- so every case
// below needs both doors open at once.

/// A value that resolved to nothing has no expression form, so the array around
/// it cannot be written down and the receiver refuses -- at every depth.
///
/// Answering a shorter list is the one thing this walk must not do: an element
/// dropped out of the array around it writes CSS the source does not describe,
/// and the two depths used to answer differently for the same value. This is
/// the reading `evaluate_result_vec_to_array_expr` already gives every other
/// caller.
#[test]
fn a_value_that_resolved_to_nothing_refuses_the_receiver() {
  for question in QUESTIONS {
    for receiver in [
      "[(() => 1) + 1]",
      "[(() => 1) + 1, 'a']",
      "[[(() => 1) + 1], 'a']",
      "[[[(() => 1) + 1]], 'a']",
      "[[(() => 1) + 1][0], 'a']",
    ] {
      assert_refuses_after_the_memo(
        question,
        &format!("sx.missing ?? {receiver}"),
        ILLEGAL_PROP_ARRAY_VALUE,
      );
    }
  }
}

/// A hole beside such a value is what the array refuses for: the array literal
/// holds a hole, so it folds to nothing at all and never reaches the receiver
/// reader.
#[test]
fn a_hole_beside_a_value_that_resolved_to_nothing_refuses_for_the_hole() {
  for question in QUESTIONS {
    for receiver in [
      "[, (() => 1) + 1]",
      "[, (() => 1) + 1, 'a']",
      "[, [(() => 1) + 1][0], 'a']",
    ] {
      assert_refuses_after_the_memo(question, receiver, PATH_WITHOUT_NODE);
    }
  }
}
