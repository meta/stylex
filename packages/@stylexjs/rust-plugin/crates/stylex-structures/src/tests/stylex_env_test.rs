//! Tests for EnvEntry accessors and JSFunction construction and invocation.

use crate::stylex_env::*;
use swc_core::ecma::ast::{Expr, Lit};

fn make_str_expr(s: &str) -> Expr {
  Expr::Lit(Lit::from(s))
}

fn make_num_expr(v: f64) -> Expr {
  Expr::Lit(Lit::from(v))
}

#[test]
fn env_entry_as_expr_returns_some_for_expr() {
  let entry = EnvEntry::Expr(make_str_expr("hello"));
  assert!(entry.as_expr().is_some());
}

#[test]
fn env_entry_as_expr_returns_none_for_function() {
  let entry = EnvEntry::Function(JSFunction::new(|_| make_num_expr(0.0)));
  assert!(entry.as_expr().is_none());
}

#[test]
fn env_entry_as_function_returns_some_for_function() {
  let entry = EnvEntry::Function(JSFunction::new(|_| make_num_expr(0.0)));
  assert!(entry.as_function().is_some());
}

#[test]
fn env_entry_as_function_returns_none_for_expr() {
  let entry = EnvEntry::Expr(make_str_expr("x"));
  assert!(entry.as_function().is_none());
}

#[test]
fn env_entry_is_function_true() {
  let entry = EnvEntry::Function(JSFunction::new(|_| make_num_expr(0.0)));
  assert!(entry.is_function());
}

#[test]
fn env_entry_is_function_false() {
  let entry = EnvEntry::Expr(make_num_expr(1.0));
  assert!(!entry.is_function());
}

#[test]
fn js_function_call_works() {
  let f = JSFunction::new(|args| {
    if args.is_empty() {
      make_num_expr(0.0)
    } else {
      args[0].clone()
    }
  });
  let result = f.call(vec![make_str_expr("input")]);
  assert!(matches!(result, Expr::Lit(Lit::Str(_))));
}

#[test]
fn js_function_debug_format() {
  let f = JSFunction::new(|_| make_num_expr(0.0));
  let debug = format!("{:?}", f);
  assert_eq!(debug, "JSFunction(<closure>)");
}
