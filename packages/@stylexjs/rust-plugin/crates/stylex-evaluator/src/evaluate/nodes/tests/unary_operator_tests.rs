//! What the prefix operators answer.
//!
//! `void`, `typeof`, `!`, `+`, `-` and `~` each read their operand a different
//! way, and three of them read it without folding it at all. Every answer here
//! is the language's own, so a declaration written with one folds to what a
//! browser would have computed.
//!
//! The values the evaluator holds of its own -- an array as its own list, the
//! compiler's function fold -- have no expression form, and each operator has
//! to read one anyway: they all stand for an object or a function upstream, so
//! `typeof` and `!` answer for them rather than refusing.

use crate::evaluate::source_evaluation::*;
use stylex_ast::ast::convertors::create_number_expr;
use stylex_constants::constants::evaluation_errors::{
  NUMERIC_CONVERSION, grown_string_too_large, unsupported_expression, unsupported_operator,
};
use stylex_constants::constants::messages::ILLEGAL_PROP_VALUE;
use stylex_state::{evaluate_result_value::EvaluateResultValue, functions::FunctionMap};
use swc_core::ecma::ast::{Expr, Lit};

/// The text `source` folds to against the compiler's own function fold, for the
/// cases whose operand is one of the values with no expression form.
#[track_caller]
fn folded_against_the_fold(fns: &FunctionMap, source: &str) -> String {
  folded_text_of(evaluated_against(fns, source), source)
}

/// `void x` is `undefined` whatever `x` is, and the operand is never folded --
/// so one that would have refused does not refuse this.
#[test]
fn void_answers_undefined_without_folding_its_operand() {
  assert_folds_to_undefined("void 0");
  assert_folds_to_undefined("void 'red'");
  assert_folds_to_undefined("void unknownName");
  assert_folds_to_undefined("void 'x'.constructor");
}

/// `typeof` of a function is `"function"` in all three spellings, and is
/// answered without folding the operand: folding one would answer for its body
/// rather than for its kind.
#[test]
fn typeof_a_function_is_function_in_every_spelling() {
  for source in [
    "typeof (() => 1)",
    "typeof (function named() { return 1; })",
    "typeof (class Named {})",
  ] {
    assert_folds_to_string(source, "function");
  }
}

/// The primitive table, at every kind the evaluator can hold.
#[test]
fn typeof_reads_the_kind_of_every_primitive() {
  assert_folds_to_string("typeof 'red'", "string");
  assert_folds_to_string("typeof 1", "number");
  assert_folds_to_string("typeof true", "boolean");
  assert_folds_to_string("typeof null", "object");
  assert_folds_to_string("typeof undefined", "undefined");
  assert_folds_to_string("typeof ({ color: 'red' })", "object");
  assert_folds_to_string("typeof [1, 2]", "object");
}

/// A value with no expression form is classified the way the object bridge
/// classifies it: the namespace is an object and one of its entries is a
/// function, exactly as they are upstream.
#[test]
fn typeof_reads_the_kind_of_a_value_with_no_expression_form() {
  let fns = a_function_fold();

  assert_eq!(
    folded_against_the_fold(&fns, &format!("typeof {FOLD_NAMESPACE}")),
    "object"
  );
  assert_eq!(
    folded_against_the_fold(&fns, &format!("typeof {FOLD_FUNCTION}")),
    "function"
  );
}

/// An operand whose kind this evaluator has no reading of refuses rather than
/// guessing a type name. `typeof` is answered off the value, so what stops it
/// is the operand's own refusal to fold, and that is the sentence an author
/// reads.
#[test]
fn typeof_an_unreadable_kind_refuses() {
  assert_deopt_reason_contains("typeof 1n", &unsupported_expression("BigIntLiteral"));
}

/// `!` reads the one truthiness table the logical operators read, so a value
/// with no expression form still answers -- every one of them stands for an
/// object, which is truthy.
#[test]
fn the_negation_reads_the_truthiness_of_every_value() {
  assert_folds_to_boolean("!''", true);
  assert_folds_to_boolean("!0", true);
  assert_folds_to_boolean("!null", true);
  assert_folds_to_boolean("!undefined", true);
  assert_folds_to_boolean("!'red'", false);
  assert_folds_to_boolean("![]", false);
  assert_folds_to_boolean("!({})", false);
  assert_folds_to_boolean("!!''", false);

  // The values the evaluator holds of its own, which is the half the doc is
  // about: an expression operand alone never reaches the arm that answers for
  // them. Each stands for an object or a function upstream, so each is truthy.
  let fns = a_function_fold();

  for source in [
    format!("!{FOLD_NAMESPACE}"),
    format!("!{FOLD_FUNCTION}"),
    "!['a'].map((entry) => entry)".to_string(),
  ] {
    match folded_value_of(evaluated_against(&fns, &source), &source).as_expr() {
      Some(Expr::Lit(Lit::Bool(answer))) => {
        assert!(!answer.value, "expected `{}` to read as truthy", source)
      },
      other => panic!(
        "expected `{}` to fold to a boolean, got {:?}",
        source, other
      ),
    }
  }
}

/// The three numeric operators, over the operands each of the two readings
/// reaches: a name and a folded expression through the first, an object or an
/// array through the second.
#[test]
fn the_numeric_operators_read_both_ways() {
  assert_folds_to_number("+'5'", 5.0);
  assert_folds_to_number("-'5'", -5.0);
  assert_folds_to_number("-(2 + 3)", -5.0);
  assert_folds_to_number("+[]", 0.0);
  assert_folds_to_number("-[5]", -5.0);
  assert_folds_to_number("~5", -6.0);
  assert_folds_to_number("~[4294967296]", -1.0);
}

/// `~` applies `ToInt32` before it negates, so the wrap happens in 32 bits.
/// A 64-bit negation would answer `-4294967297` where the language answers
/// `-1`.
#[test]
fn the_bitwise_negation_wraps_in_thirty_two_bits() {
  assert_folds_to_number("~4294967296", -1.0);
  assert_folds_to_number("~-1", 0.0);
}

/// A function has no numeric form, and the language answers `NaN` for one
/// rather than throwing -- so the operators answer it too, and the value flows
/// into the declaration exactly as it does upstream.
#[test]
fn a_numeric_operator_over_a_function_answers_not_a_number() {
  let fns = a_function_fold();

  for source in [
    format!("-{FOLD_FUNCTION}"),
    format!("+{FOLD_FUNCTION}"),
    format!("-{FOLD_NAMESPACE}"),
    format!("+{FOLD_NAMESPACE}"),
  ] {
    assert_result_folds_to_nan(evaluated_against(&fns, &source), &source);
  }
}

/// `~` is the exception, and the language's own: it applies `ToInt32` first,
/// which reads `NaN` as zero, so `~fn` is `-1` rather than `NaN`.
#[test]
fn the_bitwise_negation_of_a_function_is_minus_one() {
  let fns = a_function_fold();
  let source = format!("~{FOLD_FUNCTION}");

  assert_eq!(
    folded_value_of(evaluated_against(&fns, &source), &source),
    EvaluateResultValue::Expr(create_number_expr(-1.0))
  );
}

/// The operators that mutate or that only a statement can use are not folded at
/// all, and the refusal names the operator so an author can see which one.
#[test]
fn an_operator_the_evaluator_does_not_fold_names_itself() {
  assert_deopt_reason_contains("delete ({ a: 1 }).a", &unsupported_operator("delete"));
}

/// A value the fold carries out of a property is read for its kind like any
/// other. The two shapes that reach `typeof` only this way are an arrow and an
/// array literal: written directly, the first is answered before the operand is
/// folded at all and the second folds to the evaluator's own list.
#[test]
fn typeof_reads_the_kind_of_a_value_read_out_of_an_object() {
  assert_folds_to_string("typeof ({ a: () => 1 }).a", "function");
  assert_folds_to_string("typeof ({ a: [1, 2] }).a", "object");
}

/// An operand with no numeric reading refuses, and the sentence is the first
/// reading's own -- the coercion has nothing to add about a shape the evaluator
/// could not read at all.
///
/// `{ toString: 1 }` is such a shape, at all three operators. Its own
/// `toString` is not callable and `Object.prototype.valueOf` answers the object
/// rather than a primitive, so the language itself throws `Cannot convert
/// object to primitive value` where a number was wanted. A refusal is the
/// answer a value no runtime can produce deserves.
#[test]
fn a_numeric_operator_over_an_object_with_no_conversion_names_the_shape() {
  for source in [
    "-({ toString: 1 })",
    "+({ toString: 1 })",
    "~({ toString: 1 })",
  ] {
    assert_deopt_reason_contains(source, "ObjectExpression");
  }
}

/// The operand's number is read off a string, and that string is measured
/// against the character ceiling as it grows. The refusal names the conversion
/// the author wrote rather than the join inside it.
#[test]
fn a_numeric_operator_refuses_a_text_past_the_ceiling() {
  assert_refused_with(
    &evaluated_in_a_module_binding_under(
      a_character_ceiling_of(4),
      "digits",
      "['1234567890']",
      "-digits",
    ),
    "-digits",
    &grown_string_too_large(NUMERIC_CONVERSION, 4),
  );
}

/// An operand that *answers* nothing refuses, which is the other half of the
/// case below: there the operand is a value that is not there, and here it is a
/// subtree that resolved to nothing at all.
///
/// The two are told apart by where the refusal comes from. A warmed memo answers
/// `None` confidently for the subtree below, so the `?` in each node fires
/// without the operand having refused first, and the sentence is the one the
/// dispatch's own tail records for the operand's kind -- not the one the node
/// invents for a value it cannot read. Both spellings have to refuse, and the
/// reason each node asks one question rather than two is that the two absences
/// are the same absence.
#[test]
fn an_operator_over_an_operand_that_answered_nothing_refuses() {
  for source in [
    "typeof ((() => 1) + 1)",
    "!((() => 1) + 1)",
    "-((() => 1) + 1)",
    "((() => 1) + 1) ? 'red' : 'blue'",
  ] {
    assert_refused_with(
      &evaluated_after(UNRESOLVED_MEMO_WARM, source),
      source,
      &unsupported_expression("BinaryExpression"),
    );
  }
}

/// A value that is not there has no truthiness, no kind and no number, and each
/// operator refuses rather than guessing one. Read as "absent" it would be
/// falsy, `"undefined"` and `NaN`; read as "a value nothing resolved" it has
/// none of the three -- and the evaluator does not know which it is, so a guess
/// would put a value in the stylesheet that the source does not describe.
///
/// `??` is the exception, and answers: absence is exactly what it asks about.
#[test]
fn an_operator_over_a_value_that_is_not_there_refuses() {
  for source in [
    "![(() => 1) + 1][0]",
    "typeof [(() => 1) + 1][0]",
    "-[(() => 1) + 1][0]",
    // A conditional asks the same truthiness question the negation asks, so it
    // answers the same way. Read as `false` instead, this folded to the
    // alternate arm and wrote a value the source does not describe.
    "[(() => 1) + 1][0] ? 'red' : 'blue'",
  ] {
    assert_refused_with(
      &evaluated_after(UNRESOLVED_MEMO_WARM, source),
      source,
      ILLEGAL_PROP_VALUE,
    );
  }

  assert_eq!(
    folded_text_of(
      evaluated_after(UNRESOLVED_MEMO_WARM, "[(() => 1) + 1][0] ?? 'red'"),
      "a fallback beside a value that is not there"
    ),
    "red"
  );
}
