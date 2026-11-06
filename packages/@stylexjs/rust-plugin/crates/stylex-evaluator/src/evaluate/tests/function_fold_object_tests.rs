//! Tests object materialization of injected function maps and other compiler-held values.

use super::*;
use stylex_ast::ast::convertors::create_null_expr;
use stylex_state::{
  flat_compiled_styles_value::FlatCompiledStylesValue,
  functions::FunctionConfigType,
  types::{FlatCompiledStyles, FunctionConfigMap},
};

fn plain_config() -> FunctionConfig {
  FunctionConfig {
    fn_ptr: FunctionType::Mapper(Rc::new(create_null_expr)),
    takes_path: false,
  }
}

fn keys_of(object: &ObjectLit) -> Vec<(String, bool)> {
  object
    .props
    .iter()
    .map(|prop| {
      let key_value = prop
        .as_prop()
        .and_then(|prop| prop.as_key_value())
        .unwrap_or_else(|| panic!("expected a key-value property"));

      let key = match &key_value.key {
        PropName::Ident(ident) => ident.sym.to_string(),
        PropName::Str(strng) => convert_atom_to_string(&strng.value),
        other => panic!("expected a named key, got {:?}", other),
      };

      (key, matches!(key_value.value.as_ref(), Expr::Arrow(_)))
    })
    .collect()
}

fn entry_of(object: &ObjectLit, index: usize) -> Vec<(String, bool)> {
  let value = object.props[index]
    .as_prop()
    .and_then(|prop| prop.as_key_value())
    .map(|key_value| key_value.value.as_ref().clone())
    .unwrap_or_else(|| panic!("expected a key-value property"));

  match value {
    Expr::Object(nested) => keys_of(&nested),
    other => panic!("expected an object entry, got {:?}", other),
  }
}

#[test]
fn a_map_nested_in_a_map_carries_the_inner_maps_keys() {
  let mut nested = FunctionConfigMap::default();
  nested.insert(
    "keyframes".into(),
    FunctionConfigType::Regular(plain_config()),
  );
  nested.insert(
    "positionTry".into(),
    FunctionConfigType::Regular(plain_config()),
  );

  let mut map = FunctionConfigMap::default();
  map.insert("stylex".into(), FunctionConfigType::Map(nested));

  let object = function_fold_to_object(&EvaluateResultValue::FunctionConfigMap(map))
    .expect("a function map holding a nested map has an object form");

  assert_eq!(
    entry_of(&object, 0),
    vec![
      ("keyframes".to_string(), true),
      ("positionTry".to_string(), true)
    ]
  );
}

#[test]
fn a_compiled_style_map_carries_its_namespace_names() {
  let mut styles: FlatCompiledStyles = IndexMap::default();
  styles.insert(
    "base".to_string(),
    Rc::new(FlatCompiledStylesValue::String("x1a2b3c".to_string())),
  );
  styles.insert(
    "$$css".to_string(),
    Rc::new(FlatCompiledStylesValue::Bool(true)),
  );

  let mut map = FunctionConfigMap::default();
  map.insert(
    "styles".into(),
    FunctionConfigType::IndexMap(Rc::new(styles)),
  );

  let object = function_fold_to_object(&EvaluateResultValue::FunctionConfigMap(map))
    .expect("a function map holding a compiled style map has an object form");

  assert_eq!(
    entry_of(&object, 0),
    vec![("base".to_string(), true), ("$$css".to_string(), true)]
  );
}
