use indexmap::IndexMap;
use stylex_macros::stylex_panic;

pub const SEPARATOR: &str = ".";

#[derive(Debug, Clone, PartialEq)]
pub enum NestedStringValue {
  Str(String),
  Namespace(IndexMap<String, NestedStringValue>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum NestedConstsValue {
  Str(String),
  Num(f64),
  Namespace(IndexMap<String, NestedConstsValue>),
}

pub trait NestedNamespace {
  fn as_namespace(&self) -> Option<&IndexMap<String, Self>>
  where
    Self: Sized;
}

pub const KEY_SEPARATOR_ERROR_SUFFIX: &str = concat!(
  "must not contain the \".\" character. ",
  "Use nested objects instead of dots in key names. ",
  "See: https://www.designtokens.org/tr/drafts/format/#character-restrictions"
);

impl NestedNamespace for NestedStringValue {
  fn as_namespace(&self) -> Option<&IndexMap<String, Self>> {
    match self {
      Self::Namespace(map) => Some(map),
      Self::Str(_) => None,
    }
  }
}

impl NestedNamespace for NestedConstsValue {
  fn as_namespace(&self) -> Option<&IndexMap<String, Self>> {
    match self {
      Self::Namespace(map) => Some(map),
      Self::Str(_) | Self::Num(_) => None,
    }
  }
}

pub fn is_string_leaf(value: &NestedStringValue) -> bool {
  match value {
    NestedStringValue::Str(_) => true,
    NestedStringValue::Namespace(_) => false,
  }
}

pub fn is_consts_leaf(value: &NestedConstsValue) -> bool {
  match value {
    NestedConstsValue::Str(_) | NestedConstsValue::Num(_) => true,
    NestedConstsValue::Namespace(_) => false,
  }
}

fn flatten_nested_string_impl(
  obj: &IndexMap<String, NestedStringValue>,
  prefix: &str,
  result: &mut IndexMap<String, String>,
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
      NestedStringValue::Str(value) => {
        result.insert(full_key, value.clone());
      },
      NestedStringValue::Namespace(namespace) => {
        flatten_nested_string_impl(namespace, &full_key, result);
      },
    }
  }
}

pub fn flatten_nested_string_config(
  obj: &IndexMap<String, NestedStringValue>,
) -> IndexMap<String, String> {
  let mut result = IndexMap::new();
  flatten_nested_string_impl(obj, "", &mut result);
  result
}

#[cfg(test)]
#[path = "tests/nested_test.rs"]
mod tests;
