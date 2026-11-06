use indexmap::IndexMap;
use stylex_ast::ast::convertors::{
  convert_key_value_to_str, convert_str_lit_to_string, create_number_expr, create_string_expr,
  get_key_values_from_object, key_value_name,
};
use stylex_ast::ast::factories::{create_key_value_prop, create_object_expression};
use stylex_enums::value_with_default::ValueWithDefault;
use stylex_macros::{stylex_panic, stylex_unreachable};
use stylex_structures::base_css_type::BaseCSSType;
use stylex_structures::nested::{
  KEY_SEPARATOR_ERROR_SUFFIX, NestedConstsValue, NestedNamespace, NestedStringValue, SEPARATOR,
};
use swc_core::ecma::ast::{Expr, Lit, ObjectLit};

#[derive(Debug, Clone, PartialEq)]
pub enum NestedVarsValue {
  Str(String),
  CssType(BaseCSSType),
  Conditional(IndexMap<String, NestedVarsValue>),
  Namespace(IndexMap<String, NestedVarsValue>),
}

impl NestedNamespace for NestedVarsValue {
  fn as_namespace(&self) -> Option<&IndexMap<String, Self>> {
    match self {
      Self::Namespace(map) => Some(map),
      Self::Str(_) | Self::CssType(_) | Self::Conditional(_) => None,
    }
  }
}

pub fn is_vars_leaf(value: &NestedVarsValue) -> bool {
  match value {
    NestedVarsValue::Str(_) | NestedVarsValue::CssType(_) | NestedVarsValue::Conditional(_) => true,
    NestedVarsValue::Namespace(_) => false,
  }
}

fn flatten_nested_vars_impl(
  obj: &IndexMap<String, NestedVarsValue>,
  prefix: &str,
  result: &mut IndexMap<String, Expr>,
  transform_leaf: fn(&NestedVarsValue) -> Expr,
) {
  for (key, value) in obj {
    if key.contains(SEPARATOR) {
      stylex_panic!("Key \"{key}\" {}", KEY_SEPARATOR_ERROR_SUFFIX);
    }

    let full_key = if prefix.is_empty() {
      key.clone()
    } else {
      format!("{prefix}{SEPARATOR}{key}")
    };

    match value {
      NestedVarsValue::Str(_) | NestedVarsValue::CssType(_) | NestedVarsValue::Conditional(_) => {
        result.insert(full_key, transform_leaf(value));
      },
      NestedVarsValue::Namespace(namespace) => {
        flatten_nested_vars_impl(namespace, &full_key, result, transform_leaf);
      },
    }
  }
}

fn flatten_nested_consts_impl(
  obj: &IndexMap<String, NestedConstsValue>,
  prefix: &str,
  result: &mut IndexMap<String, Expr>,
) {
  for (key, value) in obj {
    if key.contains(SEPARATOR) {
      stylex_panic!("Key \"{key}\" {}", KEY_SEPARATOR_ERROR_SUFFIX);
    }

    let full_key = if prefix.is_empty() {
      key.clone()
    } else {
      format!("{prefix}{SEPARATOR}{key}")
    };

    match value {
      NestedConstsValue::Str(_) | NestedConstsValue::Num(_) => {
        result.insert(full_key, flatten_nested_consts_leaf(value));
      },
      NestedConstsValue::Namespace(namespace) => {
        flatten_nested_consts_impl(namespace, &full_key, result);
      },
    }
  }
}

// The `Namespace` arm is statically unreachable: the flattening helpers invoke
// the transform on values that `is_vars_leaf` accepts, which excludes Namespace.
fn transform_vars_leaf(value: &NestedVarsValue) -> Expr {
  match value {
    NestedVarsValue::Str(value) => create_string_expr(value),
    NestedVarsValue::CssType(value) => value.clone().into(),
    NestedVarsValue::Conditional(_) => to_vars_config_value(value),
    NestedVarsValue::Namespace(_) => {
      stylex_unreachable!("Nested namespace cannot be transformed as a vars leaf.")
    },
  }
}

fn transform_overrides_leaf(value: &NestedVarsValue) -> Expr {
  match value {
    NestedVarsValue::Str(value) => create_string_expr(value),
    NestedVarsValue::CssType(value) => value_with_default_to_expr(&value.value),
    NestedVarsValue::Conditional(_) => to_vars_config_value(value),
    NestedVarsValue::Namespace(_) => {
      stylex_unreachable!("Nested namespace cannot be transformed as an overrides leaf.")
    },
  }
}

pub fn to_vars_config_value(value: &NestedVarsValue) -> Expr {
  match value {
    NestedVarsValue::Str(value) => create_string_expr(value),
    NestedVarsValue::CssType(value) => value.clone().into(),
    NestedVarsValue::Conditional(map) | NestedVarsValue::Namespace(map) => {
      let props = map
        .iter()
        .map(|(key, value)| create_key_value_prop(key, to_vars_config_value(value)))
        .collect();

      create_object_expression(props)
    },
  }
}

pub fn flatten_nested_vars_config(
  obj: &IndexMap<String, NestedVarsValue>,
) -> IndexMap<String, Expr> {
  let mut result = IndexMap::new();
  flatten_nested_vars_impl(obj, "", &mut result, transform_vars_leaf);
  result
}

pub fn flatten_nested_overrides_config(
  obj: &IndexMap<String, NestedVarsValue>,
) -> IndexMap<String, Expr> {
  let mut result = IndexMap::new();
  flatten_nested_vars_impl(obj, "", &mut result, transform_overrides_leaf);
  result
}

fn flatten_nested_consts_leaf(value: &NestedConstsValue) -> Expr {
  match value {
    NestedConstsValue::Str(value) => create_string_expr(value),
    NestedConstsValue::Num(value) => create_number_expr(*value),
    NestedConstsValue::Namespace(_) => {
      stylex_unreachable!("Nested namespace cannot be transformed as a consts leaf.")
    },
  }
}

pub fn flatten_nested_consts_config(
  obj: &IndexMap<String, NestedConstsValue>,
) -> IndexMap<String, Expr> {
  let mut result = IndexMap::new();
  flatten_nested_consts_impl(obj, "", &mut result);
  result
}

pub fn object_lit_to_nested_vars_config(obj: &ObjectLit) -> IndexMap<String, NestedVarsValue> {
  get_key_values_from_object(obj)
    .into_iter()
    .filter_map(|key_value| {
      expr_to_nested_vars_value(key_value.value.as_ref())
        .map(|value| (convert_key_value_to_str(&key_value), value))
    })
    .collect()
}

pub fn object_lit_to_nested_string_config(obj: &ObjectLit) -> IndexMap<String, NestedStringValue> {
  get_key_values_from_object(obj)
    .into_iter()
    .filter_map(|key_value| {
      expr_to_nested_string_value(key_value.value.as_ref())
        .map(|value| (convert_key_value_to_str(&key_value), value))
    })
    .collect()
}

pub fn object_lit_to_nested_consts_config(obj: &ObjectLit) -> IndexMap<String, NestedConstsValue> {
  get_key_values_from_object(obj)
    .into_iter()
    .filter_map(|key_value| {
      expr_to_nested_consts_value(key_value.value.as_ref())
        .map(|value| (convert_key_value_to_str(&key_value), value))
    })
    .collect()
}

fn expr_to_nested_vars_value(expr: &Expr) -> Option<NestedVarsValue> {
  match expr {
    Expr::Lit(Lit::Str(strng)) => Some(NestedVarsValue::Str(convert_str_lit_to_string(strng))),
    Expr::Object(obj) if is_css_type_object(obj) => {
      Some(NestedVarsValue::CssType(obj.clone().into()))
    },
    Expr::Object(obj) if is_conditional_object(obj) => {
      let map = get_key_values_from_object(obj)
        .into_iter()
        .map(|key_value| {
          (
            convert_key_value_to_str(&key_value),
            to_vars_config_nested_value(key_value.value.as_ref()),
          )
        })
        .collect();

      Some(NestedVarsValue::Conditional(map))
    },
    Expr::Object(obj) => {
      let map = object_lit_to_nested_vars_config(obj);
      Some(NestedVarsValue::Namespace(map))
    },
    _ => None,
  }
}

fn expr_to_nested_string_value(expr: &Expr) -> Option<NestedStringValue> {
  match expr {
    Expr::Lit(Lit::Str(strng)) => Some(NestedStringValue::Str(convert_str_lit_to_string(strng))),
    Expr::Object(obj) => Some(NestedStringValue::Namespace(
      object_lit_to_nested_string_config(obj),
    )),
    _ => None,
  }
}

fn expr_to_nested_consts_value(expr: &Expr) -> Option<NestedConstsValue> {
  match expr {
    Expr::Lit(Lit::Str(strng)) => Some(NestedConstsValue::Str(convert_str_lit_to_string(strng))),
    Expr::Lit(Lit::Num(num)) => Some(NestedConstsValue::Num(num.value)),
    Expr::Object(obj) => Some(NestedConstsValue::Namespace(
      object_lit_to_nested_consts_config(obj),
    )),
    _ => None,
  }
}

fn to_vars_config_nested_value(expr: &Expr) -> NestedVarsValue {
  match expr {
    Expr::Lit(Lit::Str(strng)) => NestedVarsValue::Str(convert_str_lit_to_string(strng)),
    Expr::Lit(Lit::Num(num)) => NestedVarsValue::Str(num.value.to_string()),
    Expr::Lit(Lit::Bool(boolean)) => NestedVarsValue::Str(boolean.value.to_string()),
    Expr::Lit(Lit::Null(_)) => NestedVarsValue::Str(String::new()),
    Expr::Object(obj) => NestedVarsValue::Namespace(
      get_key_values_from_object(obj)
        .into_iter()
        .map(|key_value| {
          (
            convert_key_value_to_str(&key_value),
            to_vars_config_nested_value(key_value.value.as_ref()),
          )
        })
        .collect(),
    ),
    _ => NestedVarsValue::Str(String::new()),
  }
}

pub fn is_css_type_object(obj: &ObjectLit) -> bool {
  let mut has_syntax = false;
  let mut has_value = false;

  for key_value in get_key_values_from_object(obj) {
    let key = key_value_name(&key_value);
    has_syntax |= key == "syntax";
    has_value |= key == "value";
  }

  has_syntax && has_value
}

pub fn is_conditional_object(obj: &ObjectLit) -> bool {
  let key_values = get_key_values_from_object(obj);
  let has_default = key_values
    .iter()
    .any(|key_value| key_value_name(key_value) == "default");

  has_default
    && key_values.iter().all(|key_value| {
      let key = key_value_name(key_value);
      key == "default" || key.starts_with('@')
    })
}

pub fn value_with_default_to_expr(value: &ValueWithDefault) -> Expr {
  match value {
    ValueWithDefault::Number(value) => create_string_expr(&value.to_string()),
    ValueWithDefault::String(value) => create_string_expr(value),
    ValueWithDefault::Map(map) => create_object_expression(
      map
        .iter()
        .map(|(key, value)| create_key_value_prop(key, value_with_default_to_expr(value)))
        .collect(),
    ),
  }
}

#[cfg(test)]
#[path = "tests/nested_test.rs"]
mod tests;
