use std::borrow::Cow;
use std::hash::{Hash, Hasher};
use std::mem::discriminant;

use rustc_hash::FxHasher;
use serde_json::Value;
use stylex_macros::stylex_unreachable;
use stylex_utils::number::to_js_string;

use std::rc::Rc;

use swc_core::ecma::ast::Expr;

use stylex_styleq::StyleqValue;

use stylex_structures::base_css_type::BaseCSSType;
use stylex_types::structures::injectable_style::InjectableStyle;

use crate::types::FlatCompiledStyles;

/// One value a compiled style map holds.
#[derive(Debug, PartialEq, Clone)]
pub enum FlatCompiledStylesValue {
  String(String),
  /// A number an inline declaration holds.
  Number(f64),
  /// An object an inline declaration holds, such as the body of `:hover`, and the inline style itself once a merge has written it under `style`.
  Object(FlatCompiledStyles),
  /// A list an inline declaration holds, such as the fallbacks an author gives one property.
  List(Vec<Rc<FlatCompiledStylesValue>>),
  Null,
  /// A declaration the author wrote with no value.
  Undefined,
  InjectableStyle(InjectableStyle),
  Bool(bool),
  Tuple(String, Box<Expr>, Option<BaseCSSType>),
}

/// `NaN` is the one value left where the two disagree, and it disagrees the other way -- two `NaN`s are never equal and always hash alike.
impl Hash for FlatCompiledStylesValue {
  fn hash<H: Hasher>(&self, state: &mut H) {
    discriminant(self).hash(state);

    match self {
      FlatCompiledStylesValue::String(value) => value.hash(state),
      FlatCompiledStylesValue::Number(value) => zero_without_a_sign(*value).to_bits().hash(state),
      FlatCompiledStylesValue::Object(values) => {
        values.len().hash(state);

        let combined = values.iter().fold(0, |combined, (key, value)| {
          let mut entry = FxHasher::default();

          key.hash(&mut entry);
          value.hash(&mut entry);

          combined ^ entry.finish()
        });

        combined.hash(state);
      },

      FlatCompiledStylesValue::List(elements) => {
        elements.len().hash(state);

        for element in elements {
          element.hash(state);
        }
      },
      FlatCompiledStylesValue::Null | FlatCompiledStylesValue::Undefined => {},
      FlatCompiledStylesValue::InjectableStyle(value) => value.hash(state),
      FlatCompiledStylesValue::Bool(value) => value.hash(state),
      FlatCompiledStylesValue::Tuple(key, value, css_type) => {
        key.hash(state);
        value.hash(state);
        css_type.hash(state);
      },
    }
  }
}

/// The text a JavaScript object spells when a string is asked of it.
const OBJECT_AS_TEXT: &str = "[object Object]";

/// The word JavaScript writes for a value that was never given.
const UNDEFINED_AS_TEXT: &str = "undefined";

/// The text a list spells: every element in turn, with a comma between.
fn list_text(elements: &[Rc<FlatCompiledStylesValue>]) -> String {
  let mut text = String::new();

  for (index, element) in elements.iter().enumerate() {
    if index > 0 {
      text.push(',');
    }

    text.push_str(&element.as_element_text());
  }

  text
}

/// The value one JSON number holds.
#[cfg_attr(coverage_nightly, coverage(off))]
fn number_from_json(number: &serde_json::Number) -> FlatCompiledStylesValue {
  match number.as_f64() {
    Some(number) => FlatCompiledStylesValue::Number(number),
    None => FlatCompiledStylesValue::String(number.to_string()),
  }
}

/// The JSON one number spells.
fn json_number(number: f64) -> Value {
  let whole = number.fract() == 0.0 && number.abs() <= MAX_WHOLE_NUMBER;

  match whole {
    true => Value::from(number as i64),
    false => Value::from(number),
  }
}

/// The largest whole number a double holds exactly, which is JavaScript's `Number.MAX_SAFE_INTEGER`.
const MAX_WHOLE_NUMBER: f64 = 9_007_199_254_740_991.0;

fn zero_without_a_sign(value: f64) -> f64 {
  match value == 0.0 {
    true => 0.0,
    false => value,
  }
}

impl FlatCompiledStylesValue {
  pub fn as_tuple(&self) -> Option<(&String, &Expr, &Option<BaseCSSType>)> {
    match self {
      FlatCompiledStylesValue::Tuple(key, value, css_type) => Some((key, value, css_type)),
      _ => None,
    }
  }

  /// Whether this value is the number kind, and the number it holds.
  pub fn as_number(&self) -> Option<f64> {
    match self {
      FlatCompiledStylesValue::Number(number) => Some(*number),
      _ => None,
    }
  }

  pub fn as_string(&self) -> Option<&String> {
    match self {
      FlatCompiledStylesValue::String(value) => Some(value),
      _ => None,
    }
  }

  pub fn as_injectable_style(&self) -> Option<&InjectableStyle> {
    match self {
      FlatCompiledStylesValue::InjectableStyle(value) => Some(value),
      _ => None,
    }
  }

  #[allow(dead_code)]
  pub(crate) fn as_bool(&self) -> Option<&bool> {
    match self {
      FlatCompiledStylesValue::Bool(value) => Some(value),
      _ => None,
    }
  }

  #[allow(dead_code)]
  pub(crate) fn as_null(&self) -> Option<()> {
    match self {
      FlatCompiledStylesValue::Null => Some(()),
      _ => None,
    }
  }
}

impl FlatCompiledStylesValue {
  /// The JSON this value spells.
  pub fn to_json_text(&self) -> String {
    match self {
      FlatCompiledStylesValue::Number(number) if !number.is_finite() => to_js_string(*number),
      FlatCompiledStylesValue::Undefined => UNDEFINED_AS_TEXT.to_owned(),
      value => value.as_json_value().to_string(),
    }
  }

  /// The value one JSON text spells, which is the inverse of [`FlatCompiledStylesValue::to_json_text`].
  pub fn from_json_text(json_text: &str) -> Self {
    match json_text {
      UNDEFINED_AS_TEXT => FlatCompiledStylesValue::Undefined,
      "NaN" => FlatCompiledStylesValue::Number(f64::NAN),
      "Infinity" => FlatCompiledStylesValue::Number(f64::INFINITY),
      "-Infinity" => FlatCompiledStylesValue::Number(f64::NEG_INFINITY),
      json_text => match serde_json::from_str::<Value>(json_text) {
        Ok(value) => Self::from_json(&value),
        Err(_) => FlatCompiledStylesValue::String(json_text.to_owned()),
      },
    }
  }

  fn from_json(value: &Value) -> Self {
    match value {
      Value::String(text) => FlatCompiledStylesValue::String(text.clone()),
      Value::Number(number) => number_from_json(number),
      Value::Bool(value) => FlatCompiledStylesValue::Bool(*value),
      Value::Null => FlatCompiledStylesValue::Null,
      Value::Object(values) => FlatCompiledStylesValue::Object(
        values
          .iter()
          .map(|(key, value)| (key.clone(), Rc::new(Self::from_json(value))))
          .collect(),
      ),
      Value::Array(elements) => FlatCompiledStylesValue::List(
        elements
          .iter()
          .map(|element| Rc::new(Self::from_json(element)))
          .collect(),
      ),
    }
  }

  /// The text JavaScript spells for this value.
  pub fn to_js_text(&self) -> Cow<'_, str> {
    match self {
      FlatCompiledStylesValue::String(text) => Cow::Borrowed(text.as_str()),
      FlatCompiledStylesValue::Number(number) => Cow::Owned(to_js_string(*number)),
      FlatCompiledStylesValue::Bool(value) => Cow::Borrowed(match value {
        true => "true",
        false => "false",
      }),
      FlatCompiledStylesValue::Null => Cow::Borrowed("null"),
      FlatCompiledStylesValue::Undefined => Cow::Borrowed(UNDEFINED_AS_TEXT),
      FlatCompiledStylesValue::Object(_) => Cow::Borrowed(OBJECT_AS_TEXT),
      FlatCompiledStylesValue::List(elements) => Cow::Owned(list_text(elements)),
      _ => stylex_unreachable!("Encountered a value kind that spells no text."),
    }
  }

  /// The text one element of a list spells.
  fn as_element_text(&self) -> Cow<'_, str> {
    match self {
      FlatCompiledStylesValue::Null | FlatCompiledStylesValue::Undefined => Cow::Borrowed(""),
      value => value.to_js_text(),
    }
  }

  /// Convert to its JSON representation.
  pub fn as_json_value(&self) -> Value {
    match self {
      FlatCompiledStylesValue::String(text) => Value::String(text.clone()),
      FlatCompiledStylesValue::Number(number) => json_number(*number),
      FlatCompiledStylesValue::Bool(value) => Value::Bool(*value),
      FlatCompiledStylesValue::Null | FlatCompiledStylesValue::Undefined => Value::Null,
      FlatCompiledStylesValue::Object(values) => Value::Object(
        values
          .iter()
          .map(|(key, value)| (key.clone(), value.as_json_value()))
          .collect(),
      ),
      FlatCompiledStylesValue::List(elements) => Value::Array(
        elements
          .iter()
          .map(|element| element.as_json_value())
          .collect(),
      ),
      _ => stylex_unreachable!("Encountered a value kind that spells no JSON."),
    }
  }
}

impl StyleqValue for FlatCompiledStylesValue {
  fn as_class_name(&self) -> Option<&str> {
    match self {
      FlatCompiledStylesValue::String(value) => Some(value.as_str()),
      _ => None,
    }
  }

  fn is_null(&self) -> bool {
    matches!(self, FlatCompiledStylesValue::Null)
  }

  fn is_true_bool(&self) -> bool {
    matches!(self, FlatCompiledStylesValue::Bool(true))
  }

  /// An inline style is the object the author wrote, so a declaration they gave no value stands in it.
  fn is_undefined(&self) -> bool {
    matches!(self, FlatCompiledStylesValue::Undefined)
  }
}
