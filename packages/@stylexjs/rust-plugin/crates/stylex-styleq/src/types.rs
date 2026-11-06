use std::{fmt::Debug, hash::Hash, rc::Rc, sync::Arc};

use indexmap::IndexMap;

pub type StyleMap<V> = IndexMap<String, V>;
pub type Transform<V> = Arc<dyn Fn(StyleMap<V>) -> StyleMap<V> + Send + Sync>;

pub trait StyleqValue: Clone + Debug + Hash + 'static {
  fn as_class_name(&self) -> Option<&str>;
  fn is_null(&self) -> bool;
  fn is_true_bool(&self) -> bool;

  /// Whether the value stands for a property that was not given.
  ///
  /// An inline style skips such a property completely: it writes nothing,
  /// defines nothing, and leaves the property for a later style to declare.
  ///
  /// Required rather than defaulted to `false`. A default is the right answer
  /// for a value type that has no such state and the wrong one for a type that
  /// has it and forgot to say so, and the two read the same from here: the
  /// property would be written, and held against every style after it. Asking
  /// every type makes the answer a decision rather than an omission.
  fn is_undefined(&self) -> bool;
}

impl<V: StyleqValue> StyleqValue for Rc<V> {
  fn as_class_name(&self) -> Option<&str> {
    self.as_ref().as_class_name()
  }

  fn is_null(&self) -> bool {
    self.as_ref().is_null()
  }

  fn is_true_bool(&self) -> bool {
    self.as_ref().is_true_bool()
  }

  fn is_undefined(&self) -> bool {
    self.as_ref().is_undefined()
  }
}

impl<V: StyleqValue> StyleqValue for Arc<V> {
  fn as_class_name(&self) -> Option<&str> {
    self.as_ref().as_class_name()
  }

  fn is_null(&self) -> bool {
    self.as_ref().is_null()
  }

  fn is_true_bool(&self) -> bool {
    self.as_ref().is_true_bool()
  }

  fn is_undefined(&self) -> bool {
    self.as_ref().is_undefined()
  }
}

pub trait StyleqArgument<V: StyleqValue> {
  fn as_style(&self) -> Option<&StyleMap<V>>;

  /// Returns an identity key only when the style allocation outlives the cache.
  fn cache_key(&self) -> Option<usize> {
    None
  }

  fn as_nested(&self) -> Option<&[Self]>
  where
    Self: Sized,
  {
    None
  }

  fn should_skip(&self) -> bool {
    false
  }
}

#[derive(Clone)]
pub struct StyleqOptions<V: StyleqValue> {
  pub disable_cache: bool,
  pub disable_mix: bool,
  pub dedupe_class_name_chunks: bool,
  pub transform: Option<Transform<V>>,
}

impl<V: StyleqValue> Default for StyleqOptions<V> {
  fn default() -> Self {
    Self {
      disable_cache: false,
      disable_mix: false,
      dedupe_class_name_chunks: false,
      transform: None,
    }
  }
}

#[derive(Clone, Debug, PartialEq)]
pub enum StyleqInput<V: StyleqValue> {
  Style(StyleMap<V>),
  Null,
  False,
  Nested(Vec<StyleqInput<V>>),
}

impl<V: StyleqValue> StyleqArgument<V> for StyleqInput<V> {
  fn as_style(&self) -> Option<&StyleMap<V>> {
    match self {
      StyleqInput::Style(style) => Some(style),
      StyleqInput::Null | StyleqInput::False | StyleqInput::Nested(_) => None,
    }
  }

  fn as_nested(&self) -> Option<&[Self]> {
    match self {
      StyleqInput::Nested(styles) => Some(styles),
      StyleqInput::Style(_) | StyleqInput::Null | StyleqInput::False => None,
    }
  }

  fn should_skip(&self) -> bool {
    matches!(self, StyleqInput::Null | StyleqInput::False)
  }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StyleqResult<V: StyleqValue> {
  pub class_name: String,
  pub inline_style: Option<StyleMap<V>>,
  pub data_style_src: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum StyleValue {
  String(String),
  Null,
  Bool(bool),
  Number(i64),
  Object,
  Array,
  Undefined,
}

impl StyleValue {
  pub fn string(value: impl Into<String>) -> Self {
    Self::String(value.into())
  }
}

impl StyleqValue for StyleValue {
  fn as_class_name(&self) -> Option<&str> {
    match self {
      StyleValue::String(value) => Some(value),
      _ => None,
    }
  }

  fn is_null(&self) -> bool {
    matches!(self, StyleValue::Null)
  }

  fn is_true_bool(&self) -> bool {
    matches!(self, StyleValue::Bool(true))
  }

  fn is_undefined(&self) -> bool {
    matches!(self, StyleValue::Undefined)
  }
}
