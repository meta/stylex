use std::rc::Rc;

use indexmap::IndexMap;
use stylex_ast::ast::convertors::{
  create_bool_expr, create_js_number_expr, create_null_expr, create_string_expr,
};
use stylex_macros::{stylex_unimplemented, stylex_unreachable};
use swc_core::ecma::ast::{Expr, PropOrSpread};

use stylex_ast::ast::factories::{create_key_value_prop, create_object_expression};
use stylex_state::{
  flat_compiled_styles_value::FlatCompiledStylesValue,
  types::{FlatCompiledStyles, StylesObjectMap},
};

/// One named property of a compiler-written object (no spreads, methods or computed keys).
pub(crate) struct NamedProp<'a, V> {
  pub(crate) key: &'a str,
  pub(crate) value: V,
}

/// The properties of one compiled object, in the order they were written.
pub(crate) type CompiledValues<'a> = Vec<NamedProp<'a, Expr>>;

/// The namespaces of a compiled style object, each holding its own properties.
pub(crate) type CompiledNamespaces<'a> = Vec<NamedProp<'a, CompiledValues<'a>>>;

pub(crate) fn remove_objects_with_spreads(obj: &StylesObjectMap) -> StylesObjectMap {
  let mut new_obj = IndexMap::with_capacity(obj.len());

  for (key, value) in obj.iter() {
    new_obj.insert(key.clone(), value.clone());
  }

  new_obj
}

/// The properties one flat map of compiled values spells, each value keeping its kind.
fn compiled_value_props(values: &FlatCompiledStyles) -> impl Iterator<Item = NamedProp<'_, Expr>> {
  values.iter().map(|(key, value)| NamedProp {
    key: key.as_str(),
    value: compiled_value_to_ast(value),
  })
}

/// The property one named value is written as.
fn named_prop_to_key_value(prop: NamedProp<'_, Expr>) -> PropOrSpread {
  create_key_value_prop(prop.key, prop.value)
}

/// The object literal a list of named properties spells.
pub(crate) fn named_props_to_object(props: CompiledValues<'_>) -> Expr {
  create_object_expression(props.into_iter().map(named_prop_to_key_value).collect())
}

/// The object literal one flat map of compiled values spells.
pub(crate) fn convert_values_to_ast(values: &FlatCompiledStyles) -> Expr {
  create_object_expression(
    compiled_value_props(values)
      .map(named_prop_to_key_value)
      .collect(),
  )
}

/// The object literal an inline style spells; names keep their authored (camelCase) spelling.
fn convert_inline_style_to_ast(style: &FlatCompiledStyles) -> Expr {
  let props = style
    .iter()
    .map(|(key, value)| create_key_value_prop(key, compiled_value_to_ast(value)))
    .collect::<Vec<PropOrSpread>>();

  create_object_expression(props)
}

/// The expression one compiled value is written back as, keeping the kind it was read with.
pub(crate) fn compiled_value_to_ast(value: &FlatCompiledStylesValue) -> Expr {
  match value {
    FlatCompiledStylesValue::String(text) => create_string_expr(text.as_str()),
    FlatCompiledStylesValue::Number(number) => create_js_number_expr(*number),
    FlatCompiledStylesValue::Bool(value) => create_bool_expr(*value),
    FlatCompiledStylesValue::Null => create_null_expr(),
    FlatCompiledStylesValue::Object(style) => convert_inline_style_to_ast(style),
    FlatCompiledStylesValue::List(elements) => convert_list_to_ast(elements),
    // An `undefined` nested in an object or list has no output form, so refuse it.
    FlatCompiledStylesValue::Undefined => {
      stylex_unimplemented!("A style value that is undefined cannot be written.")
    },
    _ => stylex_unreachable!("Encountered an unsupported value type during AST conversion."),
  }
}

/// A list written as `{ "0": ..., "1": ... }`, as the reference does.
fn convert_list_to_ast(elements: &[Rc<FlatCompiledStylesValue>]) -> Expr {
  let props = elements
    .iter()
    .enumerate()
    .map(|(index, element)| {
      create_key_value_prop(&index.to_string(), compiled_value_to_ast(element))
    })
    .collect::<Vec<PropOrSpread>>();

  create_object_expression(props)
}

/// A whole compiled style object as named namespaces of named properties.
pub(crate) fn compiled_namespaces(namespaces: &StylesObjectMap) -> CompiledNamespaces<'_> {
  namespaces
    .iter()
    .map(|(key, values)| NamedProp {
      key: key.as_str(),
      value: compiled_value_props(values).collect(),
    })
    .collect()
}

/// The object literal a whole compiled style object spells.
pub(crate) fn namespaces_to_object(namespaces: CompiledNamespaces<'_>) -> Expr {
  create_object_expression(
    namespaces
      .into_iter()
      .map(|NamedProp { key, value }| create_key_value_prop(key, named_props_to_object(value)))
      .collect::<Vec<PropOrSpread>>(),
  )
}

#[cfg(test)]
#[path = "tests/js_to_ast_tests.rs"]
mod tests;
