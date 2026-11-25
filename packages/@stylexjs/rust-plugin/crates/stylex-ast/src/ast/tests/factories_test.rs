//! Tests for AST factory functions that create SWC expression nodes.

use crate::ast::{
  convertors::{create_number_expr, create_string_expr},
  factories::*,
};
use swc_core::ecma::ast::*;

#[test]
fn wrap_in_paren_ref_clones_and_wraps() {
  let expr = create_number_expr(5.0);
  let wrapped = wrap_in_paren_ref(&expr);
  assert!(matches!(wrapped, Expr::Paren(_)));
}

#[test]
fn create_string_array_prop_creates_array() {
  let prop = create_string_array_prop("values", &["a", "b"]);
  if let PropOrSpread::Prop(p) = prop {
    if let Prop::KeyValue(kv) = *p {
      assert!(matches!(*kv.value, Expr::Array(_)));
    } else {
      panic!("Expected KeyValue");
    }
  } else {
    panic!("Expected Prop");
  }
}

#[test]
fn create_boolean_prop_some() {
  let prop = create_boolean_prop("enabled", Some(true));
  assert!(matches!(prop, PropOrSpread::Prop(_)));
}

#[test]
fn create_number_expr_or_spread_creates() {
  let eos = create_number_expr_or_spread(3.0);
  assert!(eos.spread.is_none());
}

#[test]
fn create_array_and_spreaded_array() {
  let exprs = vec![create_string_expr("a"), create_string_expr("b")];
  let arr = create_array(&exprs);
  assert_eq!(arr.elems.len(), 2);
  assert!(arr.elems[0].as_ref().unwrap().spread.is_none());

  let spreaded = create_spreaded_array(&exprs);
  assert_eq!(spreaded.elems.len(), 2);
  assert!(spreaded.elems[0].as_ref().unwrap().spread.is_some());
}

#[test]
fn create_null_var_declarator_works() {
  let id = create_ident("y");
  let decl = create_null_var_declarator(id);
  if let Some(init) = decl.init.as_ref() {
    assert!(matches!(init.as_ref(), Expr::Lit(Lit::Null(_))));
  }
}
