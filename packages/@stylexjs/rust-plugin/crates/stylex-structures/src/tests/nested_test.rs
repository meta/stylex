//! Tests for the SWC-agnostic nested-config primitives.

use indexmap::IndexMap;

use crate::nested::{
  NestedConstsValue, NestedNamespace, NestedStringValue, SEPARATOR, flatten_nested_string_config,
  is_consts_leaf, is_string_leaf,
};

fn map<T>(entries: Vec<(&str, T)>) -> IndexMap<String, T> {
  entries
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect()
}

#[test]
fn separator_is_dot() {
  assert_eq!(SEPARATOR, ".");
}

// ---------- NestedStringValue ----------

#[test]
fn string_namespace_returns_inner_map() {
  let inner = map(vec![("a", NestedStringValue::Str("x".into()))]);
  let ns = NestedStringValue::Namespace(inner.clone());
  assert_eq!(ns.as_namespace(), Some(&inner));
}

#[test]
fn string_str_has_no_namespace() {
  let leaf = NestedStringValue::Str("hello".into());
  assert!(leaf.as_namespace().is_none());
}

#[test]
fn is_string_leaf_returns_true_for_str() {
  assert!(is_string_leaf(&NestedStringValue::Str("v".into())));
}

#[test]
fn is_string_leaf_returns_false_for_namespace() {
  let ns = NestedStringValue::Namespace(IndexMap::new());
  assert!(!is_string_leaf(&ns));
}

// ---------- NestedConstsValue ----------

#[test]
fn consts_namespace_returns_inner_map() {
  let inner = map(vec![("a", NestedConstsValue::Num(1.0))]);
  let ns = NestedConstsValue::Namespace(inner.clone());
  assert_eq!(ns.as_namespace(), Some(&inner));
}

#[test]
fn consts_str_and_num_have_no_namespace() {
  assert!(NestedConstsValue::Str("s".into()).as_namespace().is_none());
  assert!(NestedConstsValue::Num(2.5).as_namespace().is_none());
  assert!(
    NestedConstsValue::Num(std::f32::consts::PI.into())
      .as_namespace()
      .is_none()
  );
}

#[test]
fn is_consts_leaf_recognises_str_and_num() {
  assert!(is_consts_leaf(&NestedConstsValue::Str("x".into())));
  assert!(is_consts_leaf(&NestedConstsValue::Num(42.0)));
}

#[test]
fn is_consts_leaf_rejects_namespace() {
  assert!(!is_consts_leaf(&NestedConstsValue::Namespace(
    IndexMap::new()
  )));
}

// ---------- flatten_nested_string_config ----------

#[test]
fn flatten_nested_string_config_flattens_single_level() {
  let input = map(vec![
    ("a", NestedStringValue::Str("alpha".into())),
    ("b", NestedStringValue::Str("beta".into())),
  ]);
  let out = flatten_nested_string_config(&input);
  assert_eq!(out.get("a").map(String::as_str), Some("alpha"));
  assert_eq!(out.get("b").map(String::as_str), Some("beta"));
}

#[test]
fn flatten_nested_string_config_handles_empty_input() {
  let input = IndexMap::new();
  let out = flatten_nested_string_config(&input);
  assert!(out.is_empty());
}

#[test]
fn flatten_nested_string_config_joins_nested_keys_with_dot() {
  let inner = map(vec![("leaf", NestedStringValue::Str("v".into()))]);
  let input = map(vec![("outer", NestedStringValue::Namespace(inner))]);
  let out = flatten_nested_string_config(&input);
  assert_eq!(out.get("outer.leaf").map(String::as_str), Some("v"));
}

#[test]
fn flatten_nested_string_config_handles_deep_nesting() {
  let deepest = map(vec![("z", NestedStringValue::Str("end".into()))]);
  let middle = map(vec![("y", NestedStringValue::Namespace(deepest))]);
  let top = map(vec![("x", NestedStringValue::Namespace(middle))]);
  let out = flatten_nested_string_config(&top);
  assert_eq!(out.get("x.y.z").map(String::as_str), Some("end"));
}

#[test]
#[should_panic]
fn flatten_panics_on_key_with_dot() {
  let input = map(vec![("a.b", NestedStringValue::Str("nope".into()))]);
  flatten_nested_string_config(&input);
}
