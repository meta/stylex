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
///
/// Two readers share the type. A compiled namespace holds a class name, a
/// `null` for a property it clears, the compiled marker, an injectable style or
/// a variable tuple. An inline style -- a plain object a `props` call was given
/// -- holds what the author wrote instead, which is text, a number, a boolean,
/// a `null`, an absence, a list, or an object of the same kinds again.
///
#[derive(Debug, PartialEq, Clone)]
pub enum FlatCompiledStylesValue {
  String(String),
  /// A number an inline declaration holds. Kept as a number, because the value
  /// a `style` property carries is the one the author wrote: `opacity: 0.5` is
  /// a number and `opacity: '0.5'` is text.
  Number(f64),
  /// An object an inline declaration holds, such as the body of `:hover`, and
  /// the inline style itself once a merge has written it under `style`.
  Object(FlatCompiledStyles),
  /// A list an inline declaration holds, such as the fallbacks an author gives
  /// one property.
  List(Vec<Rc<FlatCompiledStylesValue>>),
  Null,
  /// A declaration the author wrote with no value. The merge drops it whole,
  /// so it declares nothing and leaves the property for a later style.
  Undefined,
  InjectableStyle(InjectableStyle),
  Bool(bool),
  Tuple(String, Box<Expr>, Option<BaseCSSType>),
}

/// Written out rather than derived, because two of the kinds hold something
/// the derived spelling reads wrongly or not at all.
///
/// A number has no `Hash` of its own, so its bits stand for it, and the two
/// zeroes are folded onto one pattern first: `-0` and `0` are one value to
/// equality, so they have to be one hash as well. `NaN` is the one value left
/// where the two disagree, and it disagrees the other way -- two `NaN`s are
/// never equal and always hash alike. The language gives no way to close that,
/// because one `NaN` cannot be told from another.
///
/// An object holds an `IndexMap`, which compares without regard to the order
/// its names were written in. The hash is combined without regard to it too,
/// so two objects that hold the same names and values stay one value. The
/// count goes in as well, so an object holding an empty object beside a `null`
/// does not write the bytes of one holding an object that holds the `null`. A
/// list is the other way round, because order is part of what a list is.
///
/// Nothing hashes an inline value today. The one caller is the merge cache in
/// `stylex-styleq`, which hashes a compiled namespace and never an inline
/// style, and this compiler runs that cache off. The rule is written here all
/// the same: nothing compares the value a hit came from, so a hash that
/// disagrees with equality answers with another style's class names.
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
      // A list keeps the order it was written in, so it hashes in that order.
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
///
/// Written into one string rather than collected and joined, because the
/// elements are spelled once and the list they would be collected into is
/// thrown away on the next line.
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
///
/// Total for every build this workspace makes. `as_f64` answers for every
/// number `serde_json` parses without the arbitrary-width feature, which
/// nothing here turns on, and a `Number` whose answer is `None` cannot be
/// built without it. The second arm is kept because the library names both
/// answers, and it is left out of the coverage measurement for that reason, as
/// `guidelines/stack/RUST.md` describes.
///
/// The attribute covers the first arm as well, because a two-arm choice cannot
/// be split across the boundary. What that arm answers is asserted anyway, by
/// the round trips in `flat_compiled_styles_value_test.rs`: a number written as
/// JSON and read back is the number it was.
#[cfg_attr(coverage_nightly, coverage(off))]
fn number_from_json(number: &serde_json::Number) -> FlatCompiledStylesValue {
  match number.as_f64() {
    Some(number) => FlatCompiledStylesValue::Number(number),
    None => FlatCompiledStylesValue::String(number.to_string()),
  }
}

/// The JSON one number spells.
///
/// A whole number spells no fraction, which is how JavaScript writes one, and
/// the two zeroes spell the same `0`. A number JSON has no word for -- the two
/// infinities and `NaN` -- spells `null`, which is what `JSON.stringify`
/// writes for it.
fn json_number(number: f64) -> Value {
  let whole = number.fract() == 0.0 && number.abs() <= MAX_WHOLE_NUMBER;

  match whole {
    true => Value::from(number as i64),
    false => Value::from(number),
  }
}

/// The largest whole number a double holds exactly, which is JavaScript's
/// `Number.MAX_SAFE_INTEGER`. Past it a whole number is written in the
/// exponent form, as the language writes it.
const MAX_WHOLE_NUMBER: f64 = 9_007_199_254_740_991.0;

/// The same number, with a negative zero folded onto the plain one.
///
/// The two are one value to equality and two bit patterns to a hash, so the
/// fold is what holds the two answers together.
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

  /// Kept beside the readers of the other kinds, so the set of them is one
  /// list rather than a list with two names missing from it. The compiler asks
  /// `styleq` about these two instead, and only the tests ask here.
  #[allow(dead_code)]
  pub(crate) fn as_bool(&self) -> Option<&bool> {
    match self {
      FlatCompiledStylesValue::Bool(value) => Some(value),
      _ => None,
    }
  }

  /// Kept for the same reason [`FlatCompiledStylesValue::as_bool`] is.
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
  ///
  /// The rule a `defineConsts` call injects carries the constant itself, and
  /// that constant crosses into JavaScript twice: once as the metadata a build
  /// tool reads, and once as the object the runtime is handed. JSON is the
  /// shape both of those are, so the kind travels as a spelling rather than as
  /// a value, and neither reader has to guess one back out of plain text.
  ///
  /// The four values JSON has no word for are spelled the way JavaScript
  /// spells them, so that a reader gets the value back rather than the `null`
  /// that `JSON.stringify` writes. Held inside an object or a list they still
  /// spell `null`: nothing reads a nested one back, and a bare `Infinity`
  /// there would leave text no reader could parse.
  pub fn to_json_text(&self) -> String {
    match self {
      FlatCompiledStylesValue::Number(number) if !number.is_finite() => to_js_string(*number),
      FlatCompiledStylesValue::Undefined => UNDEFINED_AS_TEXT.to_owned(),
      value => value.as_json_value().to_string(),
    }
  }

  /// The value one JSON text spells, which is the inverse of
  /// [`FlatCompiledStylesValue::to_json_text`].
  ///
  /// The pair exists because a `defineConsts` constant has to cross a type
  /// that cannot hold this one: the rule it is injected with is written a
  /// layer below, where the value vocabulary is not in scope. JSON carries the
  /// kind across, and this reads it back so that the readers above ask the
  /// value itself rather than the spelling.
  ///
  /// The four spellings JSON has no word for are read back first, so that a
  /// value crosses and comes back as itself. Only two readings are lost, and
  /// neither is observable: a negative zero comes back as a plain one, which
  /// is the same value and the same spelling in the language, and a value held
  /// inside an object or a list that JSON could not spell comes back as
  /// `null`, which is what `JSON.stringify` writes for it.
  ///
  /// Text that is neither reads as that text. Nothing writes such a text, and
  /// answering it keeps a constant the compiler cannot read from stopping a
  /// build over its spelling.
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
  ///
  /// Three readers ask it and each writes text: a `style` attribute, the
  /// constant an injected rule carries, and a list that joins its elements. An
  /// object spells the text every plain object spells, whatever it holds, and
  /// a list spells its elements with a comma between.
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
  ///
  /// An element that holds nothing spells nothing, which is the one rule a
  /// list adds: a value of its own names the word `null`, and a slot of a list
  /// that is null or has no value writes an empty run between two commas.
  fn as_element_text(&self) -> Cow<'_, str> {
    match self {
      FlatCompiledStylesValue::Null | FlatCompiledStylesValue::Undefined => Cow::Borrowed(""),
      value => value.to_js_text(),
    }
  }

  /// The JSON value this spells, for a reader that hands the value on rather
  /// than its text.
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

  /// An inline style is the object the author wrote, so a declaration they
  /// gave no value stands in it. A compiled style has no such value: a
  /// property the author left out is absent from the map rather than present
  /// holding nothing.
  fn is_undefined(&self) -> bool {
    matches!(self, FlatCompiledStylesValue::Undefined)
  }
}
