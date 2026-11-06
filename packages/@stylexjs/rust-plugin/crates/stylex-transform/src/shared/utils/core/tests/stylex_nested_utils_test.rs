use std::rc::Rc;

use indexmap::IndexMap;
use stylex_ast::ast::convertors::convert_lit_to_number;
use stylex_ast::ast::convertors::get_key_values_from_object;
use stylex_nested_config::nested::{NestedVarsValue, flatten_nested_vars_config};
use stylex_structures::nested::{NestedStringValue, flatten_nested_string_config};
use swc_core::ecma::ast::{Expr, Lit};

use crate::shared::utils::core::stylex_nested_utils::{
  UnflattenedCompiledStylesValue, convert_unflattened_object_to_ast, unflatten_object,
};
use stylex_state::{
  flat_compiled_styles_value::FlatCompiledStylesValue, types::FlatCompiledStyles,
};

fn flat_styles(entries: Vec<(&str, FlatCompiledStylesValue)>) -> FlatCompiledStyles {
  entries
    .into_iter()
    .map(|(key, value)| (key.to_string(), Rc::new(value)))
    .collect()
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

#[test]
#[should_panic(expected = "Key \"color.brand\" must not contain the \".\" character")]
fn string_throws_on_keys_containing_dots() {
  let input = IndexMap::from([(
    "color.brand".to_string(),
    NestedStringValue::Str("var(--x1)".to_string()),
  )]);
  let _ = flatten_nested_string_config(&input);
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
