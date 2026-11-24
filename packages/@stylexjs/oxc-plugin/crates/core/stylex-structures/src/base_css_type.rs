use indexmap::IndexMap;
use stylex_constants::constants::messages::VALUE_MUST_BE_STRING;
use stylex_enums::{css_syntax::CSSSyntax, value_with_default::ValueWithDefault};
use stylex_macros::stylex_panic;
use stylex_utils::number::to_js_string;
use stylex_value::{Object, Value};

// Carve note: #2008 built and read this type through SWC `ObjectLit`/`Expr`
// (`From<ObjectLit>`, `From<BaseCSSType> for Expr`, `get_css_value(KeyValueProp)`). Those
// conversions are front-end code now (see cut/stylex-structures/base_css_type.rs); the core
// reads and writes the same shapes as `stylex_value::Value`.

#[derive(Debug, PartialEq, Clone, Hash)]
pub struct BaseCSSType {
  pub value: ValueWithDefault,
  pub syntax: CSSSyntax,
}

/// Literal to string the way `convert_lit_to_string` read SWC literals: strings as is,
/// numbers in JavaScript's `String(n)` form, anything else refused.
fn lit_to_string(value: &Value) -> Option<String> {
  match value {
    Value::String(s) => Some(s.clone()),
    Value::Number(n) => Some(to_js_string(*n)),
    _ => None,
  }
}

impl BaseCSSType {
  /// The `{ syntax, value }` object a `stylex.types.*` call evaluates to.
  pub fn to_value(&self) -> Value {
    let mut obj = Object::new();
    obj.insert("syntax".into(), Value::String(self.syntax.to_string()));
    for (key, value) in BaseCSSType::value_to_entries(&self.value, None) {
      obj.insert(key, value);
    }
    Value::Object(obj)
  }

  pub fn value_to_entries(value: &ValueWithDefault, top_key: Option<String>) -> Vec<(String, Value)> {
    let key = top_key.unwrap_or_else(|| String::from("value"));
    match value {
      ValueWithDefault::Number(n) => vec![(key, Value::String(n.to_string()))],
      ValueWithDefault::String(s) => vec![(key, Value::String(s.clone()))],
      ValueWithDefault::Map(map) => {
        let mut local = Object::new();
        for (k, v) in map {
          local.extend(BaseCSSType::value_to_entries(v, Some(k.clone())));
        }
        vec![(key, Value::Object(local))]
      },
    }
  }

  /// Reads a `{ syntax, value }` object. Panics with #2008's messages on a malformed one.
  pub fn from_object(obj: &Object) -> BaseCSSType {
    let mut syntax: Option<CSSSyntax> = None;
    let mut values: IndexMap<String, ValueWithDefault> = IndexMap::new();

    for (key, value) in obj {
      match key.as_str() {
        "syntax" => syntax = lit_to_string(value).map(|s| s.into()),
        "value" => {
          let defaulted;
          let obj_value = match value {
            Value::Object(o) => o,
            other => {
              let Some(v) = lit_to_string(other) else {
                if matches!(other, Value::Null | Value::Bool(_)) {
                  stylex_panic!("{}", VALUE_MUST_BE_STRING);
                }
                stylex_panic!("Value must be an object or string, but got: {}", other.kind())
              };
              defaulted = Object::from_iter([("default".to_owned(), Value::String(v))]);
              &defaulted
            },
          };

          for (key, value) in obj_value {
            match value {
              Value::Object(inner) => {
                let mut obj_map = IndexMap::new();
                for (k, v) in inner {
                  match lit_to_string(v) {
                    Some(s) => obj_map.insert(k.clone(), ValueWithDefault::String(s)),
                    None => stylex_panic!("Value must be a string, but got: {}", v.kind()),
                  };
                }
                values.insert(key.clone(), ValueWithDefault::Map(obj_map));
              },
              other => match lit_to_string(other) {
                Some(s) => {
                  values.insert(key.clone(), ValueWithDefault::String(s));
                },
                None => stylex_panic!("Value must be a string or object, but got: {}", other.kind()),
              },
            }
          }
        },
        _ => stylex_panic!(r#"Key "{}" not support by BaseCSSType"#, key),
      }
    }

    assert!(!values.is_empty(), "Invalid value in defineVars");
    assert!(
      values.contains_key("default"),
      "CSS type requires a default value but none was provided."
    );

    BaseCSSType {
      value: ValueWithDefault::Map(values),
      syntax: match syntax {
        Some(s) => s,
        None => stylex_panic!("CSS syntax definition is required for this type."),
      },
    }
  }
}

/// Splits a `defineVars` value into its plain value and `@property` type: an object with both
/// `syntax` and `value` is typed, anything else comes back untyped.
pub fn get_css_value(value: &Value) -> (Value, Option<BaseCSSType>) {
  if let Value::Object(obj) = value
    && obj.contains_key("syntax")
    && let Some(inner) = obj.get("value")
  {
    return (inner.clone(), Some(BaseCSSType::from_object(obj)));
  }
  (value.clone(), None)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn round_trips_a_typed_value() {
    let input = Value::object([
      ("syntax", Value::str("<color>")),
      (
        "value",
        Value::object([
          ("default", Value::str("red")),
          ("@media (prefers-color-scheme: dark)", Value::str("white")),
        ]),
      ),
    ]);
    let (plain, ty) = get_css_value(&input);
    let ty = ty.expect("typed");
    assert_eq!(ty.syntax, CSSSyntax::Color);
    assert_eq!(plain.as_object().map(|o| o.len()), Some(2));
    assert_eq!(ty.to_value(), input);
  }

  #[test]
  fn string_value_becomes_default() {
    let input = Value::object([("syntax", Value::str("<length>")), ("value", Value::str("4px"))]);
    let ty = BaseCSSType::from_object(input.as_object().unwrap());
    assert_eq!(
      ty.value.as_map().unwrap().get("default"),
      Some(&ValueWithDefault::String("4px".into()))
    );
  }

  #[test]
  fn untyped_passes_through() {
    let v = Value::str("blue");
    assert_eq!(get_css_value(&v), (v.clone(), None));
  }

  #[test]
  #[should_panic(expected = "CSS type requires a default value")]
  fn requires_default() {
    let input = Value::object([
      ("syntax", Value::str("<color>")),
      ("value", Value::object([("x", Value::str("red"))])),
    ]);
    BaseCSSType::from_object(input.as_object().unwrap());
  }
}
