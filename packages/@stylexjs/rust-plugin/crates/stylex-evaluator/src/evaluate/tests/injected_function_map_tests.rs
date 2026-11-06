//! Tests identifier resolution through injected function maps.

use std::rc::Rc;

use indexmap::IndexMap;

use super::source_evaluation::parse_expr;
use crate::evaluate::{evaluate, fold_placeholder_function};
use stylex_state::{
  evaluate_result_value::EvaluateResultValue,
  flat_compiled_styles_value::FlatCompiledStylesValue,
  functions::{FunctionConfigType, FunctionMap},
  state_manager::StateManager,
  types::FlatCompiledStyles,
};
use stylex_structures::stylex_options::StyleXOptions;
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
