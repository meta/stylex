//! Tests type names for primitive, compiler-held and unsupported values.

use super::*;
use stylex_ast::ast::convertors::create_null_expr;
use stylex_ast::ast::factories::{create_array_expression, create_object_lit};
use swc_core::common::DUMMY_SP;
use swc_core::ecma::ast::BigInt;

#[track_caller]
fn kind_of(value: EvaluateResultValue) -> &'static str {
  match type_of(&value) {
    Some(kind) => kind,
    None => panic!("expected a kind name for {:?}", value),
  }
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
