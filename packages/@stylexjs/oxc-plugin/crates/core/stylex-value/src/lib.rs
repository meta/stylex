//! Plain JavaScript values: what a StyleX front end hands the core once it has evaluated a
//! call's arguments, and what replaces SWC's `Expr` wherever the core used to hold one as data.
//!
//! The type is parser independent on purpose. An SWC front end converts `Expr -> Value` after
//! its evaluator runs; an oxc front end does the same from its own evaluator. Nothing here can
//! name an AST node except through [`Value::Opaque`], an index into a table the front end owns.

use std::fmt;

use indexmap::IndexMap;

/// Object properties in JavaScript own-property order. The front end inserts them in that
/// order (canonical array-index keys ascending first, then string keys in insertion order);
/// [`Object::from_js_entries`] does the sorting for callers that have source order.
pub type Object = IndexMap<String, Value>;

/// Index into the front end's table of expressions the core must carry but cannot read
/// (the non-static leaves of a dynamic style function, a host value). The core copies it into
/// its output so the front end can put the original expression back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OpaqueId(pub u32);

#[derive(Debug, Clone, PartialEq, Default)]
pub enum Value {
  #[default]
  Undefined,
  Null,
  Bool(bool),
  Number(f64),
  String(String),
  Array(Vec<Value>),
  Object(Object),
  /// A value the front end could not (or must not) fold. See [`OpaqueId`].
  Opaque(OpaqueId),
}

impl Value {
  pub fn str(value: impl Into<String>) -> Self {
    Value::String(value.into())
  }

  pub fn object<K: Into<String>>(entries: impl IntoIterator<Item = (K, Value)>) -> Self {
    Value::Object(
      entries
        .into_iter()
        .map(|(key, value)| (key.into(), value))
        .collect(),
    )
  }

  pub fn as_str(&self) -> Option<&str> {
    match self {
      Value::String(s) => Some(s),
      _ => None,
    }
  }

  pub fn as_number(&self) -> Option<f64> {
    match self {
      Value::Number(n) => Some(*n),
      _ => None,
    }
  }

  pub fn as_object(&self) -> Option<&Object> {
    match self {
      Value::Object(o) => Some(o),
      _ => None,
    }
  }

  pub fn as_array(&self) -> Option<&[Value]> {
    match self {
      Value::Array(a) => Some(a),
      _ => None,
    }
  }

  pub fn is_nullish(&self) -> bool {
    matches!(self, Value::Null | Value::Undefined)
  }

  /// `typeof`-style kind name, for error messages.
  pub fn kind(&self) -> &'static str {
    match self {
      Value::Undefined => "undefined",
      Value::Null => "null",
      Value::Bool(_) => "boolean",
      Value::Number(_) => "number",
      Value::String(_) => "string",
      Value::Array(_) => "array",
      Value::Object(_) => "object",
      Value::Opaque(_) => "non-static value",
    }
  }

  /// JSON view for tests and host bridges. `undefined` and opaque values become `null`.
  pub fn to_json(&self) -> serde_json::Value {
    match self {
      Value::Undefined | Value::Null | Value::Opaque(_) => serde_json::Value::Null,
      Value::Bool(b) => serde_json::Value::Bool(*b),
      Value::Number(n) => number_to_json(*n),
      Value::String(s) => serde_json::Value::String(s.clone()),
      Value::Array(items) => serde_json::Value::Array(items.iter().map(Value::to_json).collect()),
      Value::Object(map) => serde_json::Value::Object(
        map
          .iter()
          .filter(|(_, v)| !matches!(v, Value::Undefined))
          .map(|(k, v)| (k.clone(), v.to_json()))
          .collect(),
      ),
    }
  }

  /// From JSON, keeping the JSON object's key order (`serde_json` with `preserve_order`).
  pub fn from_json(value: &serde_json::Value) -> Self {
    match value {
      serde_json::Value::Null => Value::Null,
      serde_json::Value::Bool(b) => Value::Bool(*b),
      serde_json::Value::Number(n) => Value::Number(n.as_f64().unwrap_or(f64::NAN)),
      serde_json::Value::String(s) => Value::String(s.clone()),
      serde_json::Value::Array(items) => Value::Array(items.iter().map(Value::from_json).collect()),
      serde_json::Value::Object(map) => Value::Object(
        map
          .iter()
          .map(|(k, v)| (k.clone(), Value::from_json(v)))
          .collect(),
      ),
    }
  }
}

fn number_to_json(n: f64) -> serde_json::Value {
  if !n.is_finite() {
    return serde_json::Value::Null;
  }
  if n.fract() == 0.0 && n.abs() < 9_007_199_254_740_992.0 {
    return serde_json::Value::Number((n as i64).into());
  }
  serde_json::Number::from_f64(n).map_or(serde_json::Value::Null, serde_json::Value::Number)
}

/// Whether `key` is a canonical array index (`"0"`, `"17"`, not `"01"` or `"-1"`).
pub fn is_array_index(key: &str) -> bool {
  !key.is_empty()
    && key.len() <= 10
    && key.bytes().all(|b| b.is_ascii_digit())
    && (key == "0" || !key.starts_with('0'))
    && key.parse::<u64>().is_ok_and(|n| n < u32::MAX as u64)
}

/// Builds an object from source-order entries the way JavaScript would: index keys first in
/// ascending order, the rest in insertion order, a repeated key keeping its first position
/// and its last value.
pub fn object_from_js_entries(entries: impl IntoIterator<Item = (String, Value)>) -> Object {
  let mut named = Object::new();
  let mut indexed: Vec<(u32, String, Value)> = Vec::new();
  for (key, value) in entries {
    if is_array_index(&key) {
      let n = key.parse::<u32>().unwrap_or(0);
      match indexed.iter_mut().find(|(i, _, _)| *i == n) {
        Some(slot) => slot.2 = value,
        None => indexed.push((n, key, value)),
      }
    } else {
      named.insert(key, value);
    }
  }
  indexed.sort_by_key(|(n, _, _)| *n);
  let mut out = Object::with_capacity(indexed.len() + named.len());
  out.extend(indexed.into_iter().map(|(_, k, v)| (k, v)));
  out.extend(named);
  out
}

impl fmt::Display for Value {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.to_json())
  }
}

impl From<&str> for Value {
  fn from(value: &str) -> Self {
    Value::String(value.to_owned())
  }
}

impl From<String> for Value {
  fn from(value: String) -> Self {
    Value::String(value)
  }
}

impl From<f64> for Value {
  fn from(value: f64) -> Self {
    Value::Number(value)
  }
}

impl From<bool> for Value {
  fn from(value: bool) -> Self {
    Value::Bool(value)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn js_key_order() {
    let obj = object_from_js_entries([
      ("b".to_owned(), Value::Null),
      ("2".to_owned(), Value::Null),
      ("a".to_owned(), Value::Null),
      ("1".to_owned(), Value::Null),
      ("01".to_owned(), Value::Null),
    ]);
    assert_eq!(obj.keys().collect::<Vec<_>>(), ["1", "2", "b", "a", "01"]);
  }

  #[test]
  fn json_round_trip_keeps_order() {
    let json = serde_json::json!({"z": 1, "a": [1.5, "x", null], "m": {"k": true}});
    assert_eq!(Value::from_json(&json).to_json(), json);
  }
}
