//! Writing an evaluated value back down as an expression.

use std::rc::Rc;

use indexmap::IndexMap;
use swc_core::atoms::Atom;
use swc_core::ecma::ast::{Expr, ObjectLit};

use stylex_ast::ast::convertors::create_null_expr;
use stylex_ast::ast::factories::{
  create_array_expression, create_arrow_expression, create_expr_or_spread, create_key_value_prop,
  create_object_lit,
};
use stylex_constants::constants::api_names::FUNCTION_CONFIG_FN_KEY;
use stylex_state::{
  evaluate_result_value::EvaluateResultValue,
  functions::{FunctionConfigType, FunctionType},
};
use stylex_structures::stylex_env::EnvEntry;

/// Resolves an `EnvEntry` to an `EvaluateResultValue`. - `Expr` → `EvaluateResultValue::Expr` - `Function` → returns the parent map so callers resolve the function at the call-expression site
#[inline]
pub(super) fn resolve_env_entry_to_result(
  entry: &EnvEntry,
  parent_map: &Rc<IndexMap<String, EnvEntry>>,
) -> EvaluateResultValue {
  match entry {
    EnvEntry::Host(value) => super::host_dispatch::host_result(value.clone(), None),
    EnvEntry::Expr(expr) => EvaluateResultValue::Expr(expr.clone()),

    EnvEntry::Function(_) => EvaluateResultValue::EnvObject(Rc::clone(parent_map)),
  }
}

/// Converts `EvaluateResultValue::Vec` items into an `Expr::Array`.
pub fn evaluate_result_vec_to_array_expr(items: &[EvaluateResultValue]) -> Option<Expr> {
  let mut elems = Vec::with_capacity(items.len());

  for entry in items {
    elems.push(Some(create_expr_or_spread(array_element_expr(entry)?)));
  }

  Some(create_array_expression(elems))
}

/// One element of an array, as the expression an array slot may hold.
pub(crate) fn array_element_expr(entry: &EvaluateResultValue) -> Option<Expr> {
  let expr = evaluate_result_as_expr(entry)?;

  matches!(
    expr,
    Expr::Array(_) | Expr::Object(_) | Expr::Lit(_) | Expr::Ident(_)
  )
  .then_some(expr)
}

/// The expression form of an evaluated value, if it has one.
pub(crate) fn evaluate_result_as_expr(value: &EvaluateResultValue) -> Option<Expr> {
  match value {
    EvaluateResultValue::SharedArray(values) => evaluate_result_vec_to_array_expr(values),
    EvaluateResultValue::Object(values) => {
      let mut props = Vec::new();
      for (key, value) in &values.entries {
        if matches!(value, EvaluateResultValue::Callback(_)) {
          return None;
        }
        props.push(stylex_ast::ast::factories::create_ident_key_value_prop(
          key,
          evaluate_result_as_expr(value)?,
        ));
      }
      Some(Expr::Object(create_object_lit(props)))
    },
    EvaluateResultValue::NativeArray(values) => super::native_call::native_array_expr(values),
    EvaluateResultValue::Vec(items) => evaluate_result_vec_to_array_expr(items),
    value => value.as_expr().cloned(),
  }
}

/// An object of the given keys, each carrying a function.
fn object_of_functions<'a>(keys: impl Iterator<Item = &'a str>) -> ObjectLit {
  create_object_lit(
    keys
      .map(|key| create_key_value_prop(key, fold_placeholder_function()))
      .collect(),
  )
}

/// The function a folded entry stands for.
pub(crate) fn fold_placeholder_function() -> Expr {
  create_arrow_expression(create_null_expr())
}

/// The object one entry of a folded function map stands for.
fn fold_entry_to_object(entry: &FunctionConfigType) -> ObjectLit {
  match entry {
    FunctionConfigType::Regular(config) => match &config.fn_ptr {
      FunctionType::DefaultMarker(marker_map) => {
        object_of_functions(marker_map.keys().map(String::as_str))
      },
      _ => object_of_functions(std::iter::once(FUNCTION_CONFIG_FN_KEY)),
    },

    FunctionConfigType::EnvObject(env_map) => {
      object_of_functions(env_map.keys().map(String::as_str))
    },

    FunctionConfigType::Map(nested) => object_of_functions(nested.keys().map(Atom::as_str)),
    FunctionConfigType::IndexMap(styles) => object_of_functions(styles.keys().map(String::as_str)),
  }
}

/// The object form of a folded function map, for the positions that need one.
pub fn function_fold_to_object(value: &EvaluateResultValue) -> Option<ObjectLit> {
  match value {
    EvaluateResultValue::FunctionConfigMap(func_map) => Some(create_object_lit(
      func_map
        .iter()
        .map(|(key, config)| create_key_value_prop(key, Expr::from(fold_entry_to_object(config))))
        .collect(),
    )),
    EvaluateResultValue::FunctionConfig(config) => Some(fold_entry_to_object(
      &FunctionConfigType::Regular(config.clone()),
    )),
    _ => None,
  }
}

#[cfg(test)]
pub(crate) fn binds_a_parameter(value: &EvaluateResultValue) -> bool {
  match value {
    EvaluateResultValue::ThemeRef(_) => true,
    value => evaluate_result_as_expr(value).is_some(),
  }
}
