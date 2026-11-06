//! Tests object materialization of injected function maps and other compiler-held values.

use super::*;
use stylex_ast::ast::convertors::create_null_expr;
use stylex_state::{
  flat_compiled_styles_value::FlatCompiledStylesValue,
  functions::{FunctionConfigType, StylexWhenFn},
  state_manager::StateManager,
  types::{FlatCompiledStyles, FunctionConfigMap},
};
use stylex_structures::stylex_env::EnvEntry;

fn plain_config() -> FunctionConfig {
  FunctionConfig {
    fn_ptr: FunctionType::Mapper(Rc::new(create_null_expr)),
    takes_path: false,
  }
}

fn marker_config() -> FunctionConfig {
  fn marker(
    _pseudo: EvaluateResultValue,
    _custom: Option<EvaluateResultValue>,
    _options: &mut StateManager,
  ) -> Expr {
    create_null_expr()
  }

  let mut markers: IndexMap<String, StylexWhenFn> = IndexMap::default();
  markers.insert("ancestor".to_string(), marker as StylexWhenFn);
  markers.insert("descendant".to_string(), marker as StylexWhenFn);

  FunctionConfig {
    fn_ptr: FunctionType::DefaultMarker(Arc::new(markers)),
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
fn a_plain_config_is_the_wrapper_object_the_reference_implementation_registers() {
  let object = function_fold_to_object(&EvaluateResultValue::FunctionConfig(plain_config()))
    .expect("a function config has an object form");

  assert_eq!(keys_of(&object), vec![("fn".to_string(), true)]);
}

#[test]
fn a_marker_config_carries_the_marker_names_and_not_the_wrapper_key() {
  let object = function_fold_to_object(&EvaluateResultValue::FunctionConfig(marker_config()))
    .expect("a marker config has an object form");

  let keys = keys_of(&object);

  assert!(
    keys.iter().all(|(_, is_function)| *is_function),
    "every marker is a function: {:?}",
    keys
  );
  assert!(
    keys.iter().any(|(key, _)| key == "ancestor"),
    "the marker names are the keys: {:?}",
    keys
  );
  assert!(
    !keys.iter().any(|(key, _)| key == "fn"),
    "the wrapper key is not one of them: {:?}",
    keys
  );
}

#[test]
fn a_map_carries_one_key_per_entry_and_each_entry_its_own_object() {
  let mut map = FunctionConfigMap::default();
  map.insert("when".into(), FunctionConfigType::Regular(marker_config()));
  map.insert(
    "keyframes".into(),
    FunctionConfigType::Regular(plain_config()),
  );

  let object = function_fold_to_object(&EvaluateResultValue::FunctionConfigMap(map))
    .expect("a function map has an object form");

  assert_eq!(
    keys_of(&object),
    vec![
      ("when".to_string(), false),
      ("keyframes".to_string(), false)
    ],
    "insertion order is kept and neither entry is a bare function"
  );

  assert!(
    entry_of(&object, 0)
      .iter()
      .any(|(key, _)| key == "ancestor"),
    "the marker entry is the object of its markers"
  );
  assert_eq!(entry_of(&object, 1), vec![("fn".to_string(), true)]);
}

#[test]
fn an_empty_map_materializes_to_an_object_that_would_declare_nothing() {
  let object = function_fold_to_object(&EvaluateResultValue::FunctionConfigMap(
    FunctionConfigMap::default(),
  ))
  .expect("an empty function map still has an object form");

  assert_eq!(keys_of(&object), vec![]);
}

#[test]
fn the_values_that_are_not_folds_of_a_function_have_no_object_form() {
  assert!(
    function_fold_to_object(&EvaluateResultValue::ThemeRef(ThemeRef::new(
      "vars.stylex.js",
      "vars",
      "x",
    )))
    .is_none()
  );
  assert!(function_fold_to_object(&EvaluateResultValue::Vec(vec![])).is_none());
  assert!(function_fold_to_object(&EvaluateResultValue::Map(IndexMap::default())).is_none());
  assert!(function_fold_to_object(&EvaluateResultValue::Null).is_none());
  assert!(function_fold_to_object(&EvaluateResultValue::Expr(create_null_expr())).is_none());
}

#[test]
fn an_env_object_carries_the_names_it_was_configured_with() {
  let mut env: IndexMap<String, EnvEntry> = IndexMap::default();
  env.insert("darkMode".to_string(), EnvEntry::Expr(create_null_expr()));
  env.insert("scale".to_string(), EnvEntry::Expr(create_number_expr(2.0)));

  let mut map = FunctionConfigMap::default();
  map.insert("env".into(), FunctionConfigType::EnvObject(Rc::new(env)));

  let object = function_fold_to_object(&EvaluateResultValue::FunctionConfigMap(map))
    .expect("a function map holding the env object has an object form");

  assert_eq!(
    entry_of(&object, 0),
    vec![("darkMode".to_string(), true), ("scale".to_string(), true)],
    "the configured names are the keys, in the order they were configured"
  );
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
