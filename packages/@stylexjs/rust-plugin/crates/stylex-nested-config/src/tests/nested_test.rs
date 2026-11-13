//! Tests for the SWC-aware nested-config converters.

use crate::nested::{
  NestedVarsValue, flatten_nested_consts_config, flatten_nested_overrides_config, is_vars_leaf,
  object_lit_to_nested_consts_config, object_lit_to_nested_string_config,
  object_lit_to_nested_vars_config, to_vars_config_value, value_with_default_to_expr,
};
use indexmap::IndexMap;
use stylex_ast::ast::convertors::create_number_expr;
use stylex_ast::ast::factories::{create_key_value_prop, create_object_lit, create_string_lit};
use stylex_enums::{css_syntax::CSSSyntax, value_with_default::ValueWithDefault};
use stylex_structures::base_css_type::BaseCSSType;
use stylex_structures::nested::{NestedConstsValue, NestedStringValue};
use swc_core::ecma::ast::{Expr, Lit, ObjectLit};

// ---------- helpers ----------

fn obj(props: Vec<(&str, Expr)>) -> ObjectLit {
  create_object_lit(
    props
      .into_iter()
      .map(|(k, v)| create_key_value_prop(k, v))
      .collect(),
  )
}

fn simple_css_type() -> BaseCSSType {
  let mut values = IndexMap::new();
  values.insert(
    "default".to_string(),
    ValueWithDefault::String("red".into()),
  );
  BaseCSSType {
    value: ValueWithDefault::Map(values),
    syntax: CSSSyntax::Color,
  }
}

// ---------- NestedVarsValue + is_vars_leaf ----------

#[test]
fn nested_vars_is_leaf_for_str_csstype_conditional() {
  assert!(is_vars_leaf(&NestedVarsValue::Str("v".into())));
  assert!(is_vars_leaf(&NestedVarsValue::CssType(simple_css_type())));
  assert!(is_vars_leaf(&NestedVarsValue::Conditional(IndexMap::new())));
}

#[test]
fn nested_vars_is_not_leaf_for_namespace() {
  assert!(!is_vars_leaf(&NestedVarsValue::Namespace(IndexMap::new())));
}

// ---------- to_vars_config_value ----------

#[test]
fn to_vars_config_value_css_type_into_expr() {
  let e = to_vars_config_value(&NestedVarsValue::CssType(simple_css_type()));
  assert!(matches!(e, Expr::Object(_)));
}

// ---------- flatten_nested_*_config ----------

#[test]
fn flatten_nested_overrides_config_csstype_uses_value_with_default() {
  let mut top = IndexMap::new();
  top.insert("a".to_string(), NestedVarsValue::CssType(simple_css_type()));
  let out = flatten_nested_overrides_config(&top);
  assert!(out.contains_key("a"));
  assert!(matches!(out.get("a"), Some(Expr::Object(_))));
}

// ---------- object_lit_to_nested_*_config ----------

#[test]
fn object_lit_to_nested_vars_filters_unsupported_value_types() {
  // Expr::Ident is not supported → filter_map returns None.
  let o = obj(vec![(
    "a",
    Expr::Ident(stylex_ast::ast::factories::create_ident("oops")),
  )]);
  let m = object_lit_to_nested_vars_config(&o);
  assert!(m.is_empty());
}

// ---------- is_css_type_object / is_conditional_object ----------

// ---------- value_with_default_to_expr ----------

#[test]
fn value_with_default_to_expr_number_emits_string_expr() {
  let e = value_with_default_to_expr(&ValueWithDefault::Number(2.5));
  match e {
    Expr::Lit(Lit::Str(s)) => assert!(s.value.as_str().unwrap().contains("2.5")),
    _ => panic!("expected string expr"),
  }
}

// ---------- end-to-end via to_vars_config_value with literal/number/bool/null ----------

#[test]
fn vars_config_handles_literal_value_kinds_via_conditional_object() {
  // `{ default, @… }` is conditional; the @-values hit the Lit::Num/Bool/Null arms.
  let inner = obj(vec![
    ("default", Expr::Lit(create_string_lit("d"))),
    ("@media a", Expr::Lit(create_number_lit_2(3.0))),
    ("@media b", Expr::Lit(create_bool_lit())),
    ("@media c", Expr::Lit(create_null_lit())),
  ]);
  let outer = obj(vec![("a", Expr::Object(inner))]);
  let m = object_lit_to_nested_vars_config(&outer);
  assert!(matches!(m.get("a"), Some(NestedVarsValue::Conditional(_))));
}

#[test]
fn vars_config_handles_unsupported_value_via_conditional_nested_value() {
  // Ident inside the conditional → falls into the `_ =>` arm (empty string).
  let inner = obj(vec![
    ("default", Expr::Lit(create_string_lit("d"))),
    (
      "@media x",
      Expr::Ident(stylex_ast::ast::factories::create_ident("ident")),
    ),
  ]);
  let outer = obj(vec![("a", Expr::Object(inner))]);
  let m = object_lit_to_nested_vars_config(&outer);
  assert!(matches!(m.get("a"), Some(NestedVarsValue::Conditional(_))));
}

// ---------- small literal helpers ----------

fn create_number_lit_2(value: f64) -> Lit {
  stylex_ast::ast::factories::create_number_lit(value)
}

fn create_bool_lit() -> Lit {
  stylex_ast::ast::factories::create_boolean_lit(true)
}

fn create_null_lit() -> Lit {
  stylex_ast::ast::factories::create_null_lit()
}

// ---------- additional coverage ----------

#[test]
fn nested_vars_as_namespace_returns_none_for_non_namespace_variants() {
  use stylex_structures::nested::NestedNamespace;
  assert!(NestedVarsValue::Str("x".into()).as_namespace().is_none());
  assert!(
    NestedVarsValue::CssType(simple_css_type())
      .as_namespace()
      .is_none()
  );
  assert!(
    NestedVarsValue::Conditional(IndexMap::new())
      .as_namespace()
      .is_none()
  );
}

#[test]
fn nested_vars_as_namespace_returns_map_for_namespace_variant() {
  use stylex_structures::nested::NestedNamespace;

  let value = NestedVarsValue::Namespace(IndexMap::new());

  assert!(value.as_namespace().is_some());
}

#[test]
#[should_panic(expected = "Nested namespace cannot be transformed as a vars leaf.")]
fn transform_vars_leaf_panics_for_namespace() {
  let _ = super::transform_vars_leaf(&NestedVarsValue::Namespace(IndexMap::new()));
}

#[test]
#[should_panic(expected = "Nested namespace cannot be transformed as an overrides leaf.")]
fn transform_overrides_leaf_panics_for_namespace() {
  let _ = super::transform_overrides_leaf(&NestedVarsValue::Namespace(IndexMap::new()));
}

#[test]
#[should_panic(expected = "Nested namespace cannot be transformed as a consts leaf.")]
fn flatten_nested_consts_leaf_panics_for_namespace() {
  let _ = super::flatten_nested_consts_leaf(&NestedConstsValue::Namespace(IndexMap::new()));
}

#[test]
fn object_lit_to_nested_string_config_skips_non_str_non_object() {
  // Number lit isn't supported in string-only nesting → filtered.
  let inner = obj(vec![("nope", create_number_expr(1.0))]);
  let outer = obj(vec![("n", Expr::Object(inner))]);
  let m = object_lit_to_nested_string_config(&outer);
  // The outer key still maps to a Namespace; only the inner entry is filtered.
  if let Some(NestedStringValue::Namespace(map)) = m.get("n") {
    assert!(!map.contains_key("nope"));
  } else {
    panic!("expected namespace");
  }
}

#[test]
fn object_lit_to_nested_consts_config_skips_unsupported_value_types() {
  let outer = obj(vec![(
    "x",
    Expr::Ident(stylex_ast::ast::factories::create_ident("oops")),
  )]);
  let m = object_lit_to_nested_consts_config(&outer);
  assert!(m.is_empty());
}

#[test]
#[should_panic(expected = "must not contain")]
fn flatten_nested_consts_config_panics_on_key_with_dot() {
  let mut input = IndexMap::new();
  input.insert("a.b".to_string(), NestedConstsValue::Str("nope".into()));
  let _ = flatten_nested_consts_config(&input);
}
