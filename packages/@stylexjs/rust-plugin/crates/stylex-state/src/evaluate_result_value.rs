use std::{fmt, rc::Rc};

use indexmap::IndexMap;
use serde::{
  Deserialize, Deserializer, Serialize,
  de::{Error, Visitor},
  ser::Serializer,
};
use stylex_macros::stylex_unimplemented;
use swc_core::ecma::{
  ast::{Expr, KeyValueProp, Lit, ObjectLit, Prop, PropName, PropOrSpread},
  codegen::Config,
};

use crate::{
  functions::FunctionConfig,
  theme_ref::ThemeRef,
  types::{EvaluationCallback, FunctionConfigMap},
};
use stylex_constants::constants::common::COMPILED_KEY;
use stylex_diagnostics::code_frame::{create_module, print_module};
use stylex_js::coercions::to_js_string;
use stylex_structures::stylex_env::EnvEntry;
use stylex_types::traits::WhenMarkerValue;

/// A host value and its read-time method receiver.
#[derive(Clone, Debug)]
pub struct HostedValue {
  pub value: stylex_structures::stylex_env::HostValue,
  pub receiver: stylex_structures::stylex_env::HostValue,
}
impl PartialEq for HostedValue {
  fn eq(&self, other: &Self) -> bool {
    self.value == other.value
  }
}
#[derive(Clone, Debug)]
pub struct EvaluatedObject {
  pub entries: IndexMap<String, EvaluateResultValue>,
  pub null_prototype: bool,
  pub custom_prototype: bool,
}
pub enum EvaluateResultValue {
  Null,
  Expr(Expr),
  Vec(Vec<EvaluateResultValue>),
  NativeArray(Rc<Vec<stylex_js::native::Value>>),
  SharedArray(Rc<Vec<EvaluateResultValue>>),
  Object(Rc<EvaluatedObject>),
  Map(IndexMap<Expr, Vec<KeyValueProp>>),
  Callback(EvaluationCallback),
  FunctionConfig(FunctionConfig),
  FunctionConfigMap(FunctionConfigMap),
  ThemeRef(ThemeRef),
  Host(HostedValue),
  /// An env object from the `env` config option.
  EnvObject(Rc<IndexMap<String, EnvEntry>>),
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Serialize for EvaluateResultValue {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    match self {
      Self::NativeArray(_) | Self::SharedArray(_) | Self::Object(_) => Err(
        serde::ser::Error::custom("typed values are not source serialization"),
      ),
      Self::Host(_) => Err(serde::ser::Error::custom(
        "live host values cannot be serialized as source",
      )),
      Self::Null => serializer.serialize_none(),
      Self::Expr(expr) => {
        let module = create_module(expr);

        let printed_module = print_module(
          module,
          Some(
            Config::default()
              .with_minify(true)
              .with_omit_last_semi(true)
              .with_reduce_escaped_newline(true),
          ),
        );

        serializer.serialize_str(&printed_module)
      },
      Self::Map(_) => stylex_unimplemented!("Serialization of Map values is not yet supported."),
      Self::Callback(_) => {
        stylex_unimplemented!("Serialization of Callback values is not yet supported.")
      },
      Self::FunctionConfig(_) => {
        stylex_unimplemented!("Serialization of FunctionConfig values is not yet supported.")
      },
      Self::FunctionConfigMap(_) => {
        stylex_unimplemented!("Serialization of FunctionConfigMap values is not yet supported.")
      },
      Self::ThemeRef(_) => {
        stylex_unimplemented!("Serialization of ThemeRef values is not yet supported.")
      },
      Self::Vec(_) => {
        stylex_unimplemented!("Serialization of Vec values is not yet supported.")
      },
      Self::EnvObject(_) => {
        stylex_unimplemented!("Serialization of EnvObject values is not yet supported.")
      },
    }
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl<'de> Deserialize<'de> for EvaluateResultValue {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    struct EvaluateResultValueVisitor;

    impl<'de> Visitor<'de> for EvaluateResultValueVisitor {
      type Value = EvaluateResultValue;

      fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("null")
      }

      fn visit_none<E>(self) -> Result<Self::Value, E>
      where
        E: Error,
      {
        Ok(EvaluateResultValue::Null)
      }

      fn visit_unit<E>(self) -> Result<Self::Value, E>
      where
        E: Error,
      {
        Ok(EvaluateResultValue::Null)
      }

      fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
      where
        D: Deserializer<'de>,
      {
        let _ = deserializer;
        Err(Error::custom(
          "only null EvaluateResultValue deserialization is supported",
        ))
      }
    }

    deserializer.deserialize_option(EvaluateResultValueVisitor)
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Clone for EvaluateResultValue {
  fn clone(&self) -> Self {
    match self {
      Self::Host(value) => Self::Host(value.clone()),
      Self::Null => Self::Null,
      Self::Expr(e) => Self::Expr(e.clone()),
      Self::Vec(v) => Self::Vec(v.clone()),
      Self::NativeArray(v) => Self::NativeArray(v.clone()),
      Self::SharedArray(v) => Self::SharedArray(v.clone()),
      Self::Object(v) => Self::Object(v.clone()),
      Self::Map(m) => Self::Map(m.clone()),
      Self::FunctionConfig(f) => Self::FunctionConfig(f.clone()),
      Self::FunctionConfigMap(f) => Self::FunctionConfigMap(f.clone()),
      Self::Callback(c) => Self::Callback(Rc::clone(c)),
      Self::ThemeRef(tr) => Self::ThemeRef(tr.clone()),
      Self::EnvObject(e) => Self::EnvObject(e.clone()),
    }
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl fmt::Debug for EvaluateResultValue {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Host(value) => f.debug_tuple("Host").field(value).finish(),
      Self::Null => f.debug_tuple("Null").finish(),
      Self::Expr(e) => f.debug_tuple("Expr").field(e).finish(),
      Self::Vec(v) => f.debug_tuple("Vec").field(v).finish(),
      Self::NativeArray(v) => f.debug_tuple("NativeArray").field(v).finish(),
      Self::SharedArray(v) => f.debug_tuple("SharedArray").field(v).finish(),
      Self::Object(v) => f.debug_tuple("Object").field(v).finish(),
      Self::Map(m) => f.debug_tuple("Map").field(m).finish(),
      Self::FunctionConfig(e) => f.debug_tuple("FunctionConfig").field(e).finish(),
      Self::FunctionConfigMap(e) => f.debug_tuple("FunctionConfigMap").field(e).finish(),
      Self::ThemeRef(e) => f.debug_tuple("ThemeRef").field(e).finish(),
      Self::Callback(_) => f.debug_tuple("Callback").field(&"Callback").finish(),
      Self::EnvObject(e) => f.debug_tuple("EnvObject").field(e).finish(),
    }
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl PartialEq for EvaluateResultValue {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (Self::Host(a), Self::Host(b)) => a == b,
      (Self::Null, Self::Null) => true,
      (Self::Expr(e1), Self::Expr(e2)) => e1 == e2,
      (Self::Vec(v1), Self::Vec(v2)) => v1 == v2,
      (Self::NativeArray(a), Self::NativeArray(b)) => Rc::ptr_eq(a, b),
      (Self::SharedArray(a), Self::SharedArray(b)) => Rc::ptr_eq(a, b),
      (Self::Object(a), Self::Object(b)) => Rc::ptr_eq(a, b),
      (Self::ThemeRef(v1), Self::ThemeRef(v2)) => v1 == v2,
      (Self::Map(m1), Self::Map(m2)) => m1 == m2,
      (Self::FunctionConfig(f1), Self::FunctionConfig(f2)) => f1 == f2,
      (Self::FunctionConfigMap(f1), Self::FunctionConfigMap(f2)) => f1 == f2,
      (Self::Callback(_), Self::Callback(_)) => false,
      (Self::EnvObject(_), Self::EnvObject(_)) => false,
      _ => false,
    }
  }
}

impl EvaluateResultValue {
  /// Extracts a string key from an `EvaluateResultValue::Expr` variant.
  #[inline]
  pub fn as_string_key(&self) -> Option<String> {
    match self {
      Self::Expr(expr) => string_key_of_expr(expr),
      _ => None,
    }
  }

  pub fn as_expr(&self) -> Option<&Expr> {
    match self {
      EvaluateResultValue::Expr(value) => Some(value),
      _ => None,
    }
  }

  pub fn as_vec(&self) -> Option<&Vec<EvaluateResultValue>> {
    match self {
      EvaluateResultValue::Vec(value) => Some(value),
      _ => None,
    }
  }

  pub fn as_map(&self) -> Option<&IndexMap<Expr, Vec<KeyValueProp>>> {
    match self {
      EvaluateResultValue::Map(value) => Some(value),
      _ => None,
    }
  }

  pub fn as_theme_ref(&self) -> Option<&ThemeRef> {
    match self {
      EvaluateResultValue::ThemeRef(value) => Some(value),
      _ => None,
    }
  }
}

/// Lets `stylex-css` read a marker out of an evaluated value without naming any of the evaluator's types.
impl WhenMarkerValue for EvaluateResultValue {
  fn as_str_value(&self) -> Option<&str> {
    match self {
      Self::Expr(Expr::Lit(Lit::Str(str_lit))) => str_lit.value.as_str(),
      _ => None,
    }
  }

  fn is_proxy(&self) -> bool {
    matches!(self, Self::ThemeRef(_))
  }

  fn as_proxy_string(&self) -> Option<String> {
    match self {
      Self::ThemeRef(theme_ref) => Some(theme_ref.to_string_value()),
      _ => None,
    }
  }

  fn first_css_key(&self) -> Option<&str> {
    let Self::Expr(Expr::Object(object)) = self else {
      return None;
    };

    // The second pass stops at the first non-`$$css` key — the marker class — so for the two-key object `defineMarker` emits, neither pass reads more than it must.
    let is_compiled = key_value_props(object).any(|(key, value)| {
      key == COMPILED_KEY && matches!(value, Expr::Lit(Lit::Bool(compiled)) if compiled.value)
    });

    if !is_compiled {
      return None;
    }

    key_value_props(object)
      .map(|(key, _)| key)
      .find(|key| *key != COMPILED_KEY)
  }

  fn class_name_prefix(&self) -> Option<&str> {
    None
  }
}

/// The plain key/value properties of an object literal, paired with their values.
fn key_value_props(object: &ObjectLit) -> impl Iterator<Item = (&str, &Expr)> {
  object.props.iter().filter_map(|prop| match prop {
    PropOrSpread::Prop(prop) => match prop.as_ref() {
      Prop::KeyValue(key_value) => {
        prop_name_as_str(&key_value.key).map(|key| (key, key_value.value.as_ref()))
      },
      _ => None,
    },
    PropOrSpread::Spread(_) => None,
  })
}

/// The property one expression names, which is `String(key)`.
pub fn string_key_of_expr(expr: &Expr) -> Option<String> {
  match expr {
    Expr::Ident(ident) => Some(ident.sym.to_string()),

    expr => to_js_string(expr),
  }
}

/// Reads a property name as a string, for the key shapes a compiled StyleX object can carry.
fn prop_name_as_str(key: &PropName) -> Option<&str> {
  match key {
    PropName::Ident(ident) => Some(ident.sym.as_str()),
    PropName::Str(str_lit) => str_lit.value.as_str(),
    _ => None,
  }
}

#[cfg(test)]
mod string_key_tests {
  use swc_core::{
    common::DUMMY_SP,
    ecma::ast::{Expr, Lit, Number},
  };

  use super::EvaluateResultValue;

  fn key_of(value: f64) -> Option<String> {
    EvaluateResultValue::Expr(Expr::Lit(Lit::Num(Number {
      span: DUMMY_SP,
      value,
      raw: None,
    })))
    .as_string_key()
  }

  /// A property key is `ToPropertyKey`, which is `ToString`.
  /// Rust's `Display` agrees on the ordinary magnitudes and parts company on three shapes: it signs negative zero, and it never switches to exponential form at either end of the range.
  #[test]
  fn a_number_key_is_spelled_the_way_the_language_spells_it() {
    assert_eq!(key_of(-0.0).as_deref(), Some("0"));
    assert_eq!(key_of(1e-7).as_deref(), Some("1e-7"));
    assert_eq!(key_of(1e21).as_deref(), Some("1e+21"));
  }

  #[test]
  fn an_ordinary_number_key_is_unchanged() {
    assert_eq!(key_of(0.0).as_deref(), Some("0"));
    assert_eq!(key_of(1.0).as_deref(), Some("1"));
    assert_eq!(key_of(42.5).as_deref(), Some("42.5"));
    assert_eq!(key_of(-7.0).as_deref(), Some("-7"));
  }
}
