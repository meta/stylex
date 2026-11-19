use std::rc::Rc;

use indexmap::IndexMap;
use stylex_ast::ast::convertors::get_key_values_from_object;
use stylex_ast::ast::convertors::{convert_lit_to_number, convert_lit_to_string};
use stylex_nested_config::nested::{NestedVarsValue, flatten_nested_vars_config};
use stylex_structures::nested::{NestedStringValue, flatten_nested_string_config};
use swc_core::ecma::ast::{Expr, Lit};

use crate::shared::utils::core::stylex_nested_utils::{
  UnflattenedCompiledStylesValue, convert_unflattened_object_to_ast, unflatten_object,
};
use stylex_state::{
  flat_compiled_styles_value::FlatCompiledStylesValue, types::FlatCompiledStyles,
};

fn namespace(entries: Vec<(&str, NestedVarsValue)>) -> NestedVarsValue {
  NestedVarsValue::Namespace(
    entries
      .into_iter()
      .map(|(key, value)| (key.to_string(), value))
      .collect(),
  )
}

fn flat_styles(entries: Vec<(&str, FlatCompiledStylesValue)>) -> FlatCompiledStyles {
  entries
    .into_iter()
    .map(|(key, value)| (key.to_string(), Rc::new(value)))
    .collect()
}

fn lit_string(expr: &Expr) -> String {
  match expr.as_lit().and_then(convert_lit_to_string) {
    Some(value) => value,
    None => panic!("expected string literal"),
  }
}

fn lit_number(expr: &Expr) -> f64 {
  let Some(lit) = expr.as_lit() else {
    panic!("expected number literal")
  };

  match convert_lit_to_number(lit) {
    Ok(value) => value,
    Err(_) => panic!("expected number literal"),
  }
}

fn object_prop(expr: &Expr, name: &str) -> Expr {
  let Some(obj) = expr.as_object() else {
    panic!("expected object expression")
  };

  for key_value in get_key_values_from_object(obj) {
    if stylex_ast::ast::convertors::convert_key_value_to_str(&key_value) == name {
      return key_value.value.as_ref().clone();
    }
  }

  panic!("expected object property {name}")
}

#[test]
#[should_panic(expected = "Key \"button.primary\" must not contain the \".\" character")]
fn flatten_nested_vars_throws_on_dotted_keys() {
  let input = IndexMap::from([(
    "button.primary".to_string(),
    NestedVarsValue::Str("red".to_string()),
  )]);

  let _ = flatten_nested_vars_config(&input);
}

#[test]
fn unflattened_object_converts_back_to_nested_ast() {
  let result = unflatten_object(&flat_styles(vec![
    (
      "spacing.sm",
      FlatCompiledStylesValue::String("4".to_string()),
    ),
    ("enabled", FlatCompiledStylesValue::Bool(true)),
    ("nullable", FlatCompiledStylesValue::Null),
  ]));

  let ast = convert_unflattened_object_to_ast(&result);

  assert_eq!(
    lit_number(&object_prop(&object_prop(&ast, "spacing"), "sm")),
    4.0
  );
  match object_prop(&ast, "enabled").as_lit() {
    Some(Lit::Bool(value)) => assert!(value.value),
    _ => panic!("expected bool literal"),
  }
  assert!(matches!(
    object_prop(&ast, "nullable").as_lit(),
    Some(Lit::Null(_))
  ));
}

fn conditional(entries: Vec<(&str, NestedVarsValue)>) -> NestedVarsValue {
  NestedVarsValue::Conditional(
    entries
      .into_iter()
      .map(|(key, value)| (key.to_string(), value))
      .collect(),
  )
}

fn s(value: &str) -> NestedVarsValue {
  NestedVarsValue::Str(value.to_string())
}

fn unflattened_object(
  value: &UnflattenedCompiledStylesValue,
) -> &IndexMap<String, UnflattenedCompiledStylesValue> {
  match value {
    UnflattenedCompiledStylesValue::Object(map) => map,
    UnflattenedCompiledStylesValue::Leaf(_) => panic!("expected nested object"),
  }
}

fn unflattened_leaf_string(value: &UnflattenedCompiledStylesValue) -> String {
  match value {
    UnflattenedCompiledStylesValue::Leaf(value) => match value.as_ref() {
      FlatCompiledStylesValue::String(s) => s.clone(),
      _ => panic!("expected string leaf"),
    },
    UnflattenedCompiledStylesValue::Object(_) => panic!("expected leaf, got object"),
  }
}

#[test]
fn flattens_a_deeply_nested_object_3_levels() {
  let input = IndexMap::from([(
    "button".to_string(),
    namespace(vec![
      ("primary", namespace(vec![("background", s("#00FF00"))])),
      ("secondary", namespace(vec![("background", s("#CCCCCC"))])),
    ]),
  )]);
  let result = flatten_nested_vars_config(&input);
  assert_eq!(result.len(), 2);
  assert_eq!(lit_string(&result["button.primary.background"]), "#00FF00");
  assert_eq!(
    lit_string(&result["button.secondary.background"]),
    "#CCCCCC"
  );
}

#[test]
fn flattens_4_levels_deep() {
  let input = IndexMap::from([(
    "a".to_string(),
    namespace(vec![(
      "b",
      namespace(vec![("c", namespace(vec![("d", s("value"))]))]),
    )]),
  )]);
  let result = flatten_nested_vars_config(&input);
  assert_eq!(lit_string(&result["a.b.c.d"]), "value");
  assert_eq!(result.len(), 1);
}

#[test]
fn stops_at_deeply_nested_conditional_at_rule_values() {
  let inner_cond = conditional(vec![
    ("default", s("lightblue")),
    ("@supports (color: oklch(0 0 0))", s("oklch(0.7 -0.3 -0.4)")),
  ]);
  let outer_cond = conditional(vec![
    ("default", s("blue")),
    ("@media (prefers-color-scheme: dark)", inner_cond),
  ]);
  let input = IndexMap::from([(
    "button".to_string(),
    namespace(vec![("primary", namespace(vec![("color", outer_cond)]))]),
  )]);
  let result = flatten_nested_vars_config(&input);
  assert_eq!(result.len(), 1);
  assert!(result.contains_key("button.primary.color"));
}

#[test]
#[should_panic(expected = "Key \"color.brand\" must not contain the \".\" character")]
fn string_throws_on_keys_containing_dots() {
  let input = IndexMap::from([(
    "color.brand".to_string(),
    NestedStringValue::Str("var(--x1)".to_string()),
  )]);
  let _ = flatten_nested_string_config(&input);
}

#[test]
fn handles_deeply_nested_keys_4_levels() {
  let result = unflatten_object(&flat_styles(vec![(
    "a.b.c.d",
    FlatCompiledStylesValue::String("deep".to_string()),
  )]));
  let a = unflattened_object(&result["a"]);
  let b = unflattened_object(&a["b"]);
  let c = unflattened_object(&b["c"]);
  assert_eq!(unflattened_leaf_string(&c["d"]), "deep");
}

/// The nested key wins: the leaf is what the flat spelling of the same name would write.
#[test]
fn a_nested_key_replaces_the_leaf_of_the_same_name() {
  let result = unflatten_object(&flat_styles(vec![
    (
      "button",
      FlatCompiledStylesValue::String("var(--x1)".to_string()),
    ),
    (
      "button.primary",
      FlatCompiledStylesValue::String("var(--x2)".to_string()),
    ),
  ]));

  match &result["button"] {
    UnflattenedCompiledStylesValue::Object(map) => {
      assert_eq!(map.keys().cloned().collect::<Vec<_>>(), ["primary"]);
    },
    other => panic!("the key names no object: {other:?}"),
  }
}
