//! The kind name `typeof` answers, value shape by value shape.
//!
//! Asked of the reading itself rather than through the operator, because the
//! shapes it has to name are not all shapes a source can write: the values the
//! evaluator holds of its own reach it only from inside a fold, and the two it
//! refuses reach it from nowhere at all.
//!
//! Every answer is the language's own, so a declaration written with `typeof`
//! folds to what a browser would have computed.

use super::*;
use stylex_ast::ast::convertors::{create_bool_expr, create_null_expr, create_string_expr};
use stylex_ast::ast::factories::{create_array_expression, create_object_lit};
use swc_core::common::DUMMY_SP;
use swc_core::ecma::ast::{BigInt, ThisExpr};

/// The kind `value` reads as, for a case that asserts one.
#[track_caller]
fn kind_of(value: EvaluateResultValue) -> &'static str {
  match type_of(&value) {
    Some(kind) => kind,
    None => panic!("expected a kind name for {:?}", value),
  }
}

/// The primitive table, at every kind an evaluated value can hold.
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

/// A big integer is a primitive of its own, which the `ToObject` bridge would
/// have named `object`. No source hands one over -- both compilers refuse a
/// big-integer literal before it can become a value -- but the reading is asked
/// about an expression rather than about a list of kinds, and naming this one
/// wrongly is the one way it can be wrong rather than absent.
#[test]
fn a_big_integer_reads_as_its_own_kind() {
  let big_integer = Expr::Lit(Lit::BigInt(BigInt {
    span: DUMMY_SP,
    value: Box::new(1.into()),
    raw: None,
  }));

  assert_eq!(kind_of(EvaluateResultValue::Expr(big_integer)), "bigint");
}

/// `null` is an object, and so is every other expression an evaluated value
/// holds apart from the three spellings of a function. One bridge decides
/// which, so a value read here and the same value read by a coercion cannot
/// come to disagree about what it is.
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

/// The absent value has no reading, and the refusal is what says so. Read as
/// "a value that is not there" it is `"undefined"`, and read as "a value the
/// evaluator could not carry" it is whatever that value was -- and the crate
/// settles that question in one place, the `ToObject` bridge, which refuses it.
/// Answering here would be a second answer to it.
#[test]
fn the_absent_value_has_no_reading() {
  assert_eq!(type_of(&EvaluateResultValue::Null), None);
}

/// A kind no fold produces has no reading, and the refusal is what says so.
/// `this` is such a kind: the dispatch refuses it before it can become a value,
/// so nothing hands one over -- but the reading is asked about an expression
/// rather than about a list of kinds, and a type name invented for one the
/// language reads differently would be worse than no answer.
#[test]
fn a_kind_no_fold_produces_has_no_reading() {
  let unreadable = EvaluateResultValue::Expr(Expr::This(ThisExpr { span: DUMMY_SP }));

  assert_eq!(type_of(&unreadable), None);
}
