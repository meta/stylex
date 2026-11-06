use std::rc::Rc;

use indexmap::IndexMap;
use stylex_ast::ast::factories::{
  create_key_value_prop, create_object_expression, create_string_key_value_prop,
};
use stylex_constants::constants::common::{COMPILED_KEY, VAR_GROUP_HASH_KEY};
use stylex_macros::stylex_unreachable;
use stylex_structures::nested::SEPARATOR;
use swc_core::ecma::ast::Expr;

use crate::shared::utils::core::js_to_ast::compiled_value_to_ast;

use stylex_state::{
  evaluate_result_value::EvaluateResultValue, flat_compiled_styles_value::FlatCompiledStylesValue,
  types::FlatCompiledStyles,
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum UnflattenedCompiledStylesValue {
  Leaf(Rc<FlatCompiledStylesValue>),
  Object(IndexMap<String, UnflattenedCompiledStylesValue>),
}

pub(crate) fn unflatten_object(
  flat_obj: &FlatCompiledStyles,
) -> IndexMap<String, UnflattenedCompiledStylesValue> {
  let mut result = IndexMap::new();

  for (key, value) in flat_obj {
    if is_special_key(key) || !key.contains(SEPARATOR) {
      result.insert(
        key.clone(),
        UnflattenedCompiledStylesValue::Leaf(value.clone()),
      );
      continue;
    }

    // A split always answers at least one part, so the last of them is the leaf
    // and every part before it names an object to go down into.
    let parts = key.split(SEPARATOR).collect::<Vec<_>>();
    let leaf_key = parts[parts.len() - 1];
    let mut current = &mut result;

    for part in &parts[..parts.len() - 1] {
      let entry = current
        .entry((*part).to_string())
        .or_insert_with(|| UnflattenedCompiledStylesValue::Object(IndexMap::new()));

      // A leaf already written under this part cannot hold the parts below it,
      // so it gives way to the object they go in.
      if !matches!(entry, UnflattenedCompiledStylesValue::Object(_)) {
        *entry = UnflattenedCompiledStylesValue::Object(IndexMap::new());
      }

      current = object_under(entry);
    }

    current.insert(
      leaf_key.to_string(),
      UnflattenedCompiledStylesValue::Leaf(value.clone()),
    );
  }

  result
}

pub(crate) fn expr_map_to_evaluate_result(map: IndexMap<String, Expr>) -> EvaluateResultValue {
  EvaluateResultValue::Expr(create_object_expression(
    map
      .into_iter()
      .map(|(key, value)| create_key_value_prop(&key, value))
      .collect(),
  ))
}

pub(crate) fn string_map_to_evaluate_result(map: IndexMap<String, String>) -> EvaluateResultValue {
  EvaluateResultValue::Expr(create_object_expression(
    map
      .into_iter()
      .map(|(key, value)| create_string_key_value_prop(&key, &value))
      .collect(),
  ))
}

pub(crate) fn convert_unflattened_object_to_ast(
  obj: &IndexMap<String, UnflattenedCompiledStylesValue>,
) -> Expr {
  create_object_expression(
    obj
      .iter()
      .map(|(key, value)| create_key_value_prop(key, unflattened_value_to_ast(value)))
      .collect(),
  )
}

/// The object `value` holds.
///
/// Total for the one caller above, which makes `value` an object on the line
/// before it asks. The second arm answers for a state no input can put the
/// value in, and is kept because the language names every variant or none. Left
/// out of the coverage measurement for that reason, as
/// `guidelines/stack/RUST.md` describes.
#[cfg_attr(coverage_nightly, coverage(off))]
fn object_under(
  value: &mut UnflattenedCompiledStylesValue,
) -> &mut IndexMap<String, UnflattenedCompiledStylesValue> {
  match value {
    UnflattenedCompiledStylesValue::Object(map) => map,
    UnflattenedCompiledStylesValue::Leaf(_) => {
      stylex_unreachable!("Expected unflattened intermediate object.")
    },
  }
}

/// The expression one unflattened value is written back as.
///
/// A leaf is written by the one writer every other compiled value goes
/// through, so a nested constant and a flat one are spelled the same way.
fn unflattened_value_to_ast(value: &UnflattenedCompiledStylesValue) -> Expr {
  match value {
    UnflattenedCompiledStylesValue::Object(map) => convert_unflattened_object_to_ast(map),
    UnflattenedCompiledStylesValue::Leaf(value) => compiled_value_to_ast(value),
  }
}

fn is_special_key(key: &str) -> bool {
  key == VAR_GROUP_HASH_KEY || key == COMPILED_KEY
}
