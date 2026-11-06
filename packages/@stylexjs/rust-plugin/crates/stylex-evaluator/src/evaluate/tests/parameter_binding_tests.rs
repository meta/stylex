//! Tests argument binding through expression forms and variable-group factories.

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

#[test]
fn a_value_that_writes_itself_down_has_an_expression_form() {
  for value in [
    EvaluateResultValue::Expr(create_string_expr("red")),
    EvaluateResultValue::Expr(create_number_expr(1.0)),
    EvaluateResultValue::Expr(create_null_expr()),
    EvaluateResultValue::Vec(vec![EvaluateResultValue::Expr(create_string_expr("a"))]),
    EvaluateResultValue::Vec(vec![]),
  ] {
    let expected = printed(&expression_of(&value));

    match evaluate_result_as_expr(&value) {
      Some(form) => assert_eq!(printed(&form), expected, "wrong form for {:?}", value),
      None => panic!("expected {:?} to have an expression form", value),
    }

    assert!(binds_a_parameter(&value));
  }
}

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

#[test]
fn a_theme_reference_binds_without_an_expression_form() {
  let value = EvaluateResultValue::ThemeRef(theme_ref());

  assert!(evaluate_result_as_expr(&value).is_none());
  assert!(binds_a_parameter(&value));
}

#[test]
fn a_theme_reference_inside_an_array_has_no_expression_form() {
  let value = EvaluateResultValue::Vec(vec![EvaluateResultValue::ThemeRef(theme_ref())]);

  assert!(evaluate_result_as_expr(&value).is_none());
  assert!(!binds_a_parameter(&value));
}

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

#[test]
fn the_evaluators_own_values_have_no_expression_form() {
  assert!(evaluate_result_as_expr(&EvaluateResultValue::Map(IndexMap::default())).is_none());
  assert!(evaluate_result_as_expr(&EvaluateResultValue::Null).is_none());
  assert!(!binds_a_parameter(&EvaluateResultValue::Map(
    IndexMap::default()
  )));
  assert!(!binds_a_parameter(&EvaluateResultValue::Null));
}
