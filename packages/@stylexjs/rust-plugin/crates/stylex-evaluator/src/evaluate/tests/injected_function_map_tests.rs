//! Tests identifier resolution through injected function maps.

use std::rc::Rc;
use std::sync::Arc;

use indexmap::IndexMap;

use super::source_evaluation::{FOLD_ENTRY, a_folded_function, folded_entry, parse_expr};
use crate::evaluate::{evaluate, fold_placeholder_function};
use stylex_ast::ast::convertors::{create_null_expr, create_string_expr};
use stylex_state::{
  evaluate_result_value::EvaluateResultValue,
  flat_compiled_styles_value::FlatCompiledStylesValue,
  functions::{FunctionConfigType, FunctionMap, FunctionType},
  state_manager::StateManager,
  theme_ref::ThemeRef,
  types::{FlatCompiledStyles, FunctionConfigMap},
};
use stylex_structures::{stylex_env::EnvEntry, stylex_options::StyleXOptions};
use swc_core::common::{GLOBALS, Globals};

fn resolved(entry: FunctionConfigType) -> Option<EvaluateResultValue> {
  let globals = Globals::new();

  GLOBALS.set(&globals, || {
    let mut fns = FunctionMap::default();

    fns.identifiers.insert("token".into(), Box::new(entry));

    let mut traversal_state = StateManager::new(StyleXOptions::default());
    let result = evaluate(&parse_expr("token"), &mut traversal_state, &fns);

    assert!(result.confident, "`token` refused: {:?}", result.reason);

    result.value
  })
}

#[test]
fn a_mapper_folds_to_the_value_it_stands_for() {
  let value = resolved(folded_entry(
    FunctionType::Mapper(Rc::new(|| create_string_expr("red"))),
    false,
  ));

  assert_eq!(
    value,
    Some(EvaluateResultValue::Expr(create_string_expr("red"))),
    "expected the mapper's own value"
  );
}

#[test]
fn a_theme_reference_mapper_folds_to_the_group_it_names() {
  let theme = ThemeRef::new("vars.stylex.js", "vars", "x");
  let value = resolved(folded_entry(
    FunctionType::ThemeRefMapper(Rc::new(move || theme.clone())),
    false,
  ));

  assert!(
    matches!(value, Some(EvaluateResultValue::ThemeRef(_))),
    "expected the theme reference, got {:?}",
    value
  );
}

#[test]
fn every_callable_shape_folds_to_the_callable() {
  for fn_ptr in [
    FunctionType::ArrayArgs(|_, _, _| create_null_expr()),
    FunctionType::StylexExprFn(|expr, _| expr),
    FunctionType::StylexWhenFn(|_, _, _| create_null_expr()),
    FunctionType::StylexTypeFn(Rc::new(|_| create_null_expr())),
    FunctionType::StylexFnsFactory(|_| Rc::new(|_| create_null_expr())),
    FunctionType::Callback(Box::new(create_null_expr())),
    FunctionType::DefaultMarker(Arc::new(IndexMap::default())),
  ] {
    let value = resolved(folded_entry(fn_ptr, false));

    assert!(
      matches!(value, Some(EvaluateResultValue::FunctionConfig(_))),
      "expected the callable itself, got {:?}",
      value
    );
  }
}

#[test]
fn a_map_of_entries_folds_to_the_namespace() {
  let mut map = FunctionConfigMap::default();

  map.insert(FOLD_ENTRY.into(), a_folded_function());

  let value = resolved(FunctionConfigType::Map(map));

  assert!(
    matches!(value, Some(EvaluateResultValue::FunctionConfigMap(_))),
    "expected the namespace map, got {:?}",
    value
  );
}

#[test]
fn an_index_map_folds_to_a_function() {
  let mut styles: FlatCompiledStyles = IndexMap::default();

  styles.insert("color".to_string(), Rc::new(FlatCompiledStylesValue::Null));

  let value = resolved(FunctionConfigType::IndexMap(Rc::new(styles)));

  assert_eq!(
    value,
    Some(EvaluateResultValue::Expr(fold_placeholder_function())),
    "expected the placeholder function"
  );
}

#[test]
fn an_env_object_folds_to_the_configured_object() {
  let mut env: IndexMap<String, EnvEntry> = IndexMap::default();

  env.insert(
    "breakpoint".to_string(),
    EnvEntry::Expr(create_string_expr("40rem")),
  );

  let value = resolved(FunctionConfigType::EnvObject(Rc::new(env)));

  assert!(
    matches!(value, Some(EvaluateResultValue::EnvObject(_))),
    "expected the env object, got {:?}",
    value
  );
}
