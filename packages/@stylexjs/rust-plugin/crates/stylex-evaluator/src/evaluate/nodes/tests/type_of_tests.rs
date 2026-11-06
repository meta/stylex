//! Tests type names for primitive, compiler-held and unsupported values.

use super::*;
use stylex_ast::ast::convertors::{create_bool_expr, create_null_expr, create_string_expr};
use stylex_ast::ast::factories::{create_array_expression, create_object_lit};
use swc_core::common::DUMMY_SP;
use swc_core::ecma::ast::{BigInt, ThisExpr};

#[track_caller]
fn kind_of(value: EvaluateResultValue) -> &'static str {
  match type_of(&value) {
    Some(kind) => kind,
    None => panic!("expected a kind name for {:?}", value),
  }
}

#[test]
fn every_primitive_reads_as_its_own_kind() {
  assert_eq!(
    kind_of(EvaluateResultValue::Expr(create_string_expr("a"))),
    "string"
  );
  assert_eq!(
    kind_of(EvaluateResultValue::Expr(create_number_expr(1.0))),
    "number"
  );
  assert_eq!(
    kind_of(EvaluateResultValue::Expr(create_bool_expr(true))),
    "boolean"
  );
  assert_eq!(kind_of(js_undefined()), "undefined");
}

#[test]
fn a_big_integer_reads_as_its_own_kind() {
  let big_integer = Expr::Lit(Lit::BigInt(BigInt {
    span: DUMMY_SP,
    value: Box::new(1.into()),
    raw: None,
  }));

  assert_eq!(kind_of(EvaluateResultValue::Expr(big_integer)), "bigint");
}

#[test]
fn an_expression_reads_as_the_object_or_function_it_is() {
  assert_eq!(
    kind_of(EvaluateResultValue::Expr(create_null_expr())),
    "object"
  );
  assert_eq!(
    kind_of(EvaluateResultValue::Expr(create_object_lit(vec![]).into())),
    "object"
  );
  assert_eq!(
    kind_of(EvaluateResultValue::Expr(create_array_expression(vec![]))),
    "object"
  );
  assert_eq!(
    kind_of(EvaluateResultValue::Expr(fold_placeholder_function())),
    "function"
  );
}

#[test]
fn the_absent_value_has_no_reading() {
  assert_eq!(type_of(&EvaluateResultValue::Null), None);
}

#[test]
fn a_kind_no_fold_produces_has_no_reading() {
  let unreadable = EvaluateResultValue::Expr(Expr::This(ThisExpr { span: DUMMY_SP }));

  assert_eq!(type_of(&unreadable), None);
}
