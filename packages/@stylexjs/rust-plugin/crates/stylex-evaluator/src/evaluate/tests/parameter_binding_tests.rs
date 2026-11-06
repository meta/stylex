//! Which arguments an author's own arrow binds a parameter to.
//!
//! The binding reads two forms and nothing else: the expression a value writes
//! itself down as, and — for a theme reference, which writes none — the factory a
//! module's own token import binds through. An argument with neither leaves its
//! parameter unbound, which is what the language does with a missing argument
//! too, so the body answers on its own terms and the call is not failed for the
//! argument's sake.
//!
//! One question rather than two, because the caller that names a refusal and the
//! binding that reads an argument have to agree about which arguments bound: a
//! value one accepted and the other dropped would name the wrong half of the
//! call. What the reading has to get right is the evaluator's two spellings of an
//! array — a reader that knows only the literal finds no form for half the arrays
//! it is handed.

use indexmap::IndexMap;
use swc_core::ecma::ast::ExprOrSpread;

use crate::evaluate::{binds_a_parameter, evaluate_result_as_expr};
use stylex_state::{
  evaluate_result_value::EvaluateResultValue,
  functions::{FunctionConfig, FunctionType},
  theme_ref::ThemeRef,
};

use super::source_evaluation::printed;
use stylex_ast::ast::{
  convertors::{create_null_expr, create_number_expr, create_string_expr},
  factories::{create_array_expression, create_expr_or_spread},
};
use swc_core::ecma::ast::Expr;

fn theme_ref() -> ThemeRef {
  ThemeRef::new("vars.stylex.js", "vars", "x")
}

/// The ordinary case: a value with an expression form binds as that expression.
#[test]
fn a_value_that_writes_itself_down_has_an_expression_form() {
  for value in [
    EvaluateResultValue::Expr(create_string_expr("red")),
    EvaluateResultValue::Expr(create_number_expr(1.0)),
    EvaluateResultValue::Expr(create_null_expr()),
    EvaluateResultValue::Vec(vec![EvaluateResultValue::Expr(create_string_expr("a"))]),
    EvaluateResultValue::Vec(vec![]),
  ] {
    // The expression form rather than "there is one": a form that answered
    // some other value would bind the parameter to something the source does
    // not describe.
    let expected = printed(&expression_of(&value));

    match evaluate_result_as_expr(&value) {
      Some(form) => assert_eq!(printed(&form), expected, "wrong form for {:?}", value),
      None => panic!("expected {:?} to have an expression form", value),
    }

    assert!(binds_a_parameter(&value));
  }
}

/// The expression a value of this case *is*, spelled without going through the
/// reading under test.
fn expression_of(value: &EvaluateResultValue) -> Expr {
  match value {
    EvaluateResultValue::Expr(expr) => expr.clone(),
    EvaluateResultValue::Vec(items) => create_array_expression(
      items
        .iter()
        .map(|item| Some(create_expr_or_spread(expression_of(item))))
        .collect(),
    ),
    other => panic!("no case builds {:?}", other),
  }
}

/// The evaluator's own list and the literal it was written as are one array with
/// two spellings, and both have to answer the same form.
#[test]
fn both_spellings_of_an_array_have_the_same_form() {
  let written = EvaluateResultValue::Expr(create_array_expression(vec![Some(ExprOrSpread {
    spread: None,
    expr: Box::new(create_string_expr("a")),
  })]));
  let listed = EvaluateResultValue::Vec(vec![EvaluateResultValue::Expr(create_string_expr("a"))]);

  assert_eq!(
    evaluate_result_as_expr(&written),
    evaluate_result_as_expr(&listed)
  );
}

/// A theme reference has no expression form and binds anyway, through the
/// factory a module's own token import binds through — which is the whole of why
/// the binding names it apart rather than asking this question alone.
#[test]
fn a_theme_reference_binds_without_an_expression_form() {
  let value = EvaluateResultValue::ThemeRef(theme_ref());

  assert!(evaluate_result_as_expr(&value).is_none());
  assert!(binds_a_parameter(&value));
}

/// A theme reference nested inside an array has no form either, because the
/// array writes itself down as one expression and a reference has none to
/// contribute. The parameter is then left unbound rather than holding an array
/// with a hole where the source wrote a value.
#[test]
fn a_theme_reference_inside_an_array_has_no_expression_form() {
  let value = EvaluateResultValue::Vec(vec![EvaluateResultValue::ThemeRef(theme_ref())]);

  assert!(evaluate_result_as_expr(&value).is_none());
  assert!(!binds_a_parameter(&value));
}

/// A function has no form either way. The reference compiler folds one as its
/// source text and this compiler keeps none, so the parameter stays unbound and
/// the body answers for whatever it does with the name.
#[test]
fn a_function_has_no_expression_form() {
  let config = FunctionConfig {
    fn_ptr: FunctionType::Callback(Box::new(create_null_expr())),
    takes_path: false,
  };

  assert!(evaluate_result_as_expr(&EvaluateResultValue::FunctionConfig(config.clone())).is_none());
  assert!(!binds_a_parameter(&EvaluateResultValue::FunctionConfig(
    config
  )));
}

/// The remaining values the evaluator has of its own. None writes an expression
/// down, and none has a second spelling the reading could reach.
#[test]
fn the_evaluators_own_values_have_no_expression_form() {
  assert!(evaluate_result_as_expr(&EvaluateResultValue::Map(IndexMap::default())).is_none());
  assert!(evaluate_result_as_expr(&EvaluateResultValue::Null).is_none());
  assert!(!binds_a_parameter(&EvaluateResultValue::Map(
    IndexMap::default()
  )));
  assert!(!binds_a_parameter(&EvaluateResultValue::Null));
}
