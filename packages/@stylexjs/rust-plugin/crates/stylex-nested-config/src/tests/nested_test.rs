//! Tests for the SWC-aware nested-config converters.

use crate::nested::{
  NestedVarsValue, flatten_nested_consts_config, flatten_nested_overrides_config,
  flatten_nested_vars_config, is_conditional_object, is_css_type_object, is_vars_leaf,
  object_lit_to_nested_consts_config, object_lit_to_nested_string_config,
  object_lit_to_nested_vars_config, to_vars_config_value, value_with_default_to_expr,
};
use indexmap::IndexMap;
use stylex_ast::ast::convertors::{convert_lit_to_string, create_number_expr, create_string_expr};
use stylex_ast::ast::factories::{create_key_value_prop, create_object_lit, create_string_lit};
use stylex_enums::{css_syntax::CSSSyntax, value_with_default::ValueWithDefault};
use stylex_structures::base_css_type::BaseCSSType;
use stylex_structures::nested::{NestedConstsValue, NestedStringValue};
use swc_core::ecma::ast::{Expr, Lit, ObjectLit, PropName};

// ---------- helpers ----------

fn obj(props: Vec<(&str, Expr)>) -> ObjectLit {
  create_object_lit(
    props
      .into_iter()
      .map(|(k, v)| create_key_value_prop(k, v))
      .collect(),
  )
}

fn into_obj(e: Expr) -> ObjectLit {
  match e {
    Expr::Object(o) => o,
    _ => panic!("expected object expr"),
  }
}

fn keys(o: &ObjectLit) -> Vec<String> {
  o.props
    .iter()
    .filter_map(|p| p.as_prop()?.as_key_value().map(|kv| &kv.key))
    .map(|k| match k {
      PropName::Str(s) => s.value.as_str().unwrap_or("").to_string(),
      PropName::Ident(i) => i.sym.to_string(),
      _ => "?".into(),
    })
    .collect()
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
fn to_vars_config_value_str() {
  let e = to_vars_config_value(&NestedVarsValue::Str("hi".into()));
  assert!(matches!(e, Expr::Lit(Lit::Str(_))));
}

#[test]
fn to_vars_config_value_css_type_into_expr() {
  let e = to_vars_config_value(&NestedVarsValue::CssType(simple_css_type()));
  assert!(matches!(e, Expr::Object(_)));
}

#[test]
fn to_vars_config_value_conditional_emits_object() {
  let mut map = IndexMap::new();
  map.insert("default".to_string(), NestedVarsValue::Str("a".into()));
  map.insert("@media print".to_string(), NestedVarsValue::Str("b".into()));
  let e = to_vars_config_value(&NestedVarsValue::Conditional(map));
  let o = into_obj(e);
  assert_eq!(keys(&o).len(), 2);
}

#[test]
fn to_vars_config_value_namespace_emits_object() {
  let mut inner = IndexMap::new();
  inner.insert("x".to_string(), NestedVarsValue::Str("v".into()));
  let e = to_vars_config_value(&NestedVarsValue::Namespace(inner));
  assert!(matches!(e, Expr::Object(_)));
}

// ---------- flatten_nested_*_config ----------

#[test]
fn flatten_nested_vars_config_handles_leaf_and_nested() {
  let mut inner = IndexMap::new();
  inner.insert("z".to_string(), NestedVarsValue::Str("deep".into()));
  let mut top = IndexMap::new();
  top.insert("a".to_string(), NestedVarsValue::Str("v".into()));
  top.insert("g".to_string(), NestedVarsValue::Namespace(inner));
  let out = flatten_nested_vars_config(&top);
  assert!(out.contains_key("a"));
  assert!(out.contains_key("g.z"));
}

#[test]
fn flatten_nested_overrides_config_csstype_uses_value_with_default() {
  let mut top = IndexMap::new();
  top.insert("a".to_string(), NestedVarsValue::CssType(simple_css_type()));
  let out = flatten_nested_overrides_config(&top);
  // value_with_default_to_expr should yield a string expr (since value is a Map with a default).
  assert!(out.contains_key("a"));
  assert!(matches!(out.get("a"), Some(Expr::Object(_))));
}

#[test]
fn flatten_nested_overrides_config_str_and_conditional() {
  let mut cond = IndexMap::new();
  cond.insert("default".to_string(), NestedVarsValue::Str("a".into()));
  let mut top = IndexMap::new();
  top.insert("s".to_string(), NestedVarsValue::Str("v".into()));
  top.insert("c".to_string(), NestedVarsValue::Conditional(cond));
  let out = flatten_nested_overrides_config(&top);
  assert!(out.contains_key("s"));
  assert!(out.contains_key("c"));
}

#[test]
fn flatten_nested_consts_config_emits_string_and_number_exprs() {
  let mut top = IndexMap::new();
  top.insert("s".to_string(), NestedConstsValue::Str("hi".into()));
  top.insert("n".to_string(), NestedConstsValue::Num(3.5));
  let out = flatten_nested_consts_config(&top);
  assert!(matches!(out.get("s"), Some(Expr::Lit(Lit::Str(_)))));
  assert!(matches!(out.get("n"), Some(Expr::Lit(Lit::Num(_)))));
}

#[test]
fn flatten_nested_consts_config_recurses_into_namespace() {
  let mut inner = IndexMap::new();
  inner.insert("z".to_string(), NestedConstsValue::Num(1.0));
  let mut top = IndexMap::new();
  top.insert("ns".to_string(), NestedConstsValue::Namespace(inner));
  let out = flatten_nested_consts_config(&top);
  assert!(out.contains_key("ns.z"));
}

// ---------- object_lit_to_nested_*_config ----------

#[test]
fn object_lit_to_nested_vars_config_str_value() {
  let o = obj(vec![("a", Expr::Lit(create_string_lit("v")))]);
  let m = object_lit_to_nested_vars_config(&o);
  assert!(matches!(m.get("a"), Some(NestedVarsValue::Str(_))));
}

#[test]
fn object_lit_to_nested_vars_config_recognises_css_type_object() {
  // { syntax: "<color>", value: "red" } → CssType.
  let inner = obj(vec![
    ("syntax", Expr::Lit(create_string_lit("<color>"))),
    ("value", Expr::Lit(create_string_lit("red"))),
  ]);
  let outer = obj(vec![("a", Expr::Object(inner))]);
  let m = object_lit_to_nested_vars_config(&outer);
  assert!(matches!(m.get("a"), Some(NestedVarsValue::CssType(_))));
}

#[test]
fn object_lit_to_nested_vars_config_recognises_conditional_object() {
  // { default: "x", "@media print": "y" } → Conditional.
  let inner = obj(vec![
    ("default", Expr::Lit(create_string_lit("x"))),
    ("@media print", Expr::Lit(create_string_lit("y"))),
  ]);
  let outer = obj(vec![("a", Expr::Object(inner))]);
  let m = object_lit_to_nested_vars_config(&outer);
  assert!(matches!(m.get("a"), Some(NestedVarsValue::Conditional(_))));
}

#[test]
fn object_lit_to_nested_vars_config_namespace_for_arbitrary_object() {
  // { foo: "v" } at nested position → Namespace.
  let inner = obj(vec![("foo", Expr::Lit(create_string_lit("v")))]);
  let outer = obj(vec![("a", Expr::Object(inner))]);
  let m = object_lit_to_nested_vars_config(&outer);
  assert!(matches!(m.get("a"), Some(NestedVarsValue::Namespace(_))));
}

#[test]
fn object_lit_to_nested_string_config_str_and_namespace() {
  let inner = obj(vec![("leaf", Expr::Lit(create_string_lit("v")))]);
  let outer = obj(vec![
    ("s", Expr::Lit(create_string_lit("hi"))),
    ("n", Expr::Object(inner)),
  ]);
  let m = object_lit_to_nested_string_config(&outer);
  assert!(matches!(m.get("s"), Some(NestedStringValue::Str(_))));
  assert!(matches!(m.get("n"), Some(NestedStringValue::Namespace(_))));
}

#[test]
fn object_lit_to_nested_consts_config_str_num_and_namespace() {
  let inner = obj(vec![("z", Expr::Lit(create_string_lit("v")))]);
  let outer = obj(vec![
    ("s", Expr::Lit(create_string_lit("hi"))),
    ("n", create_number_expr(1.5)),
    ("g", Expr::Object(inner)),
  ]);
  let m = object_lit_to_nested_consts_config(&outer);
  assert!(matches!(m.get("s"), Some(NestedConstsValue::Str(_))));
  assert!(matches!(m.get("n"), Some(NestedConstsValue::Num(_))));
  assert!(matches!(m.get("g"), Some(NestedConstsValue::Namespace(_))));
}

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

#[test]
fn is_css_type_object_true_when_both_keys_present() {
  let o = obj(vec![
    ("syntax", Expr::Lit(create_string_lit("<color>"))),
    ("value", Expr::Lit(create_string_lit("red"))),
  ]);
  assert!(is_css_type_object(&o));
}

#[test]
fn is_css_type_object_false_when_only_one_key_present() {
  let only_syntax = obj(vec![("syntax", Expr::Lit(create_string_lit("<color>")))]);
  let only_value = obj(vec![("value", Expr::Lit(create_string_lit("red")))]);
  let empty = obj(vec![]);
  assert!(!is_css_type_object(&only_syntax));
  assert!(!is_css_type_object(&only_value));
  assert!(!is_css_type_object(&empty));
}

#[test]
fn is_conditional_object_true_with_default_and_at_keys() {
  let o = obj(vec![
    ("default", Expr::Lit(create_string_lit("a"))),
    ("@media print", Expr::Lit(create_string_lit("b"))),
  ]);
  assert!(is_conditional_object(&o));
}

#[test]
fn is_conditional_object_false_without_default() {
  let o = obj(vec![("@media print", Expr::Lit(create_string_lit("a")))]);
  assert!(!is_conditional_object(&o));
}

#[test]
fn is_conditional_object_false_with_non_at_key() {
  let o = obj(vec![
    ("default", Expr::Lit(create_string_lit("a"))),
    ("hover", Expr::Lit(create_string_lit("b"))),
  ]);
  assert!(!is_conditional_object(&o));
}

// ---------- value_with_default_to_expr ----------

#[test]
fn value_with_default_to_expr_number_emits_string_expr() {
  let e = value_with_default_to_expr(&ValueWithDefault::Number(2.5));
  match e {
    Expr::Lit(Lit::Str(s)) => assert!(s.value.as_str().unwrap().contains("2.5")),
    _ => panic!("expected string expr"),
  }
}

#[test]
fn value_with_default_to_expr_string_emits_string_expr() {
  let e = value_with_default_to_expr(&ValueWithDefault::String("red".into()));
  assert!(matches!(e, Expr::Lit(Lit::Str(_))));
}

#[test]
fn value_with_default_to_expr_map_recurses() {
  let mut inner = IndexMap::new();
  inner.insert(
    "default".to_string(),
    ValueWithDefault::String("red".into()),
  );
  let e = value_with_default_to_expr(&ValueWithDefault::Map(inner));
  assert!(matches!(e, Expr::Object(_)));
}

// ---------- end-to-end via to_vars_config_value with literal/number/bool/null ----------

#[test]
fn vars_config_handles_literal_value_kinds_via_conditional_object() {
  // is_conditional_object is satisfied by { default, @… }; the @-branch values
  // exercise to_vars_config_nested_value's Lit::Num/Bool/Null arms.
  let inner = obj(vec![
    ("default", Expr::Lit(create_string_lit("d"))),
    ("@media a", Expr::Lit(create_number_lit_2(3.0))),
    ("@media b", Expr::Lit(create_bool_lit())),
    ("@media c", Expr::Lit(create_null_lit())),
  ]);
  let outer = obj(vec![("a", Expr::Object(inner))]);
  let m = object_lit_to_nested_vars_config(&outer);
  // It should be classified as Conditional.
  assert!(matches!(m.get("a"), Some(NestedVarsValue::Conditional(_))));
}

#[test]
fn vars_config_handles_object_value_via_conditional_nested_value() {
  let nested = obj(vec![("k", Expr::Lit(create_string_lit("v")))]);
  let inner = obj(vec![
    ("default", Expr::Lit(create_string_lit("d"))),
    ("@media x", Expr::Object(nested)),
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

#[test]
fn _sanity_convert_lit_to_string_through_consts_parsing() {
  // Ensure convert_lit_to_string (used by object_lit_to_nested_string/vars) is
  // exercised here so coverage attributes the call to this crate's tests too.
  let lit = create_string_lit("hello");
  assert_eq!(convert_lit_to_string(&lit), Some("hello".into()));
  let _ = create_string_expr("x");
  let _ = create_number_expr(1.0);
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
fn flatten_nested_vars_config_covers_csstype_leaf() {
  let mut top = IndexMap::new();
  top.insert("c".to_string(), NestedVarsValue::CssType(simple_css_type()));
  let out = flatten_nested_vars_config(&top);
  assert!(matches!(out.get("c"), Some(Expr::Object(_))));
}

#[test]
fn flatten_nested_vars_config_covers_conditional_leaf() {
  let mut cond = IndexMap::new();
  cond.insert("default".to_string(), NestedVarsValue::Str("d".into()));
  cond.insert("@media print".to_string(), NestedVarsValue::Str("p".into()));
  let mut top = IndexMap::new();
  top.insert("a".to_string(), NestedVarsValue::Conditional(cond));
  let out = flatten_nested_vars_config(&top);
  // Conditional leaf produces a single nested Expr::Object under key "a".
  assert!(matches!(out.get("a"), Some(Expr::Object(_))));
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
  // The outer key "n" still maps to a Namespace, but the inner "nope" is filtered.
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
fn flatten_nested_vars_config_panics_on_key_with_dot() {
  let mut input = IndexMap::new();
  input.insert("a.b".to_string(), NestedVarsValue::Str("nope".into()));
  let _ = flatten_nested_vars_config(&input);
}

#[test]
#[should_panic(expected = "must not contain")]
fn flatten_nested_consts_config_panics_on_key_with_dot() {
  let mut input = IndexMap::new();
  input.insert("a.b".to_string(), NestedConstsValue::Str("nope".into()));
  let _ = flatten_nested_consts_config(&input);
}
