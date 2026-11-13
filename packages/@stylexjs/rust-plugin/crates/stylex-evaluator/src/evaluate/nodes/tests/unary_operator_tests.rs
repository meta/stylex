//! Tests unary folding, operand coercion and refusal propagation.

use crate::evaluate::source_evaluation::*;
use stylex_ast::ast::convertors::create_number_expr;
use stylex_constants::constants::evaluation_errors::{
  NUMERIC_CONVERSION, grown_string_too_large, unsupported_expression, unsupported_operator,
};
use stylex_state::{evaluate_result_value::EvaluateResultValue, functions::FunctionMap};
use swc_core::ecma::ast::{Expr, Lit};

#[track_caller]
fn folded_against_the_fold(fns: &FunctionMap, source: &str) -> String {
  folded_text_of(evaluated_against(fns, source), source)
}

#[test]
fn void_answers_undefined_without_folding_its_operand() {
  assert_folds_to_undefined("void 0");
  assert_folds_to_undefined("void 'red'");
  assert_folds_to_undefined("void unknownName");
  assert_folds_to_undefined("void 'x'.constructor");
}

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

#[test]
fn typeof_an_unreadable_kind_refuses() {
  assert_deopt_reason_contains("typeof 1n", &unsupported_expression("BigIntLiteral"));
}

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

#[test]
fn the_bitwise_negation_wraps_in_thirty_two_bits() {
  assert_folds_to_number("~4294967296", -1.0);
  assert_folds_to_number("~-1", 0.0);
}

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

#[test]
fn the_bitwise_negation_of_a_function_is_minus_one() {
  let fns = a_function_fold();
  let source = format!("~{FOLD_FUNCTION}");

  assert_eq!(
    folded_value_of(evaluated_against(&fns, &source), &source),
    EvaluateResultValue::Expr(create_number_expr(-1.0))
  );
}

#[test]
fn an_operator_the_evaluator_does_not_fold_names_itself() {
  assert_deopt_reason_contains("delete ({ a: 1 }).a", &unsupported_operator("delete"));
}

#[test]
fn typeof_reads_the_kind_of_a_value_read_out_of_an_object() {
  assert_folds_to_string("typeof ({ a: () => 1 }).a", "function");
  assert_folds_to_string("typeof ({ a: [1, 2] }).a", "object");
}

#[test]
fn a_numeric_operator_over_an_object_with_no_conversion_refuses_in_the_languages_words() {
  for source in [
    "-({ toString: 1 })",
    "+({ toString: 1 })",
    "~({ toString: 1 })",
  ] {
    assert_deopt_reason_contains(source, "Cannot convert object to primitive value");
  }
}

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

#[test]
fn an_operator_over_a_value_that_is_not_there_refuses() {
  for source in [
    "![(() => 1) + 1][0]",
    "typeof [(() => 1) + 1][0]",
    "-[(() => 1) + 1][0]",
    "[(() => 1) + 1][0] ? 'red' : 'blue'",
  ] {
    assert_refused_with(
      &evaluated_after(UNRESOLVED_MEMO_WARM, source),
      source,
      &unsupported_expression("BinaryExpression"),
    );
  }

  assert_refused_with(
    &evaluated_after(UNRESOLVED_MEMO_WARM, "[(() => 1) + 1][0] ?? 'red'"),
    "a fallback beside a value the memo refused",
    &unsupported_expression("BinaryExpression"),
  );
}
