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

pub enum EvaluateResultValue {
  Null,
  Expr(Expr),
  Vec(Vec<EvaluateResultValue>),
  Map(IndexMap<Expr, Vec<KeyValueProp>>),
  Callback(EvaluationCallback),
  FunctionConfig(FunctionConfig),
  FunctionConfigMap(FunctionConfigMap),
  ThemeRef(ThemeRef),
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
      Self::Null => Self::Null,
      Self::Expr(e) => Self::Expr(e.clone()),
      Self::Vec(v) => Self::Vec(v.clone()),
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
      Self::Null => f.debug_tuple("Null").finish(),
      Self::Expr(e) => f.debug_tuple("Expr").field(e).finish(),
      Self::Vec(v) => f.debug_tuple("Vec").field(v).finish(),
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
      (Self::Null, Self::Null) => true,
      (Self::Expr(e1), Self::Expr(e2)) => e1 == e2,
      (Self::Vec(v1), Self::Vec(v2)) => v1 == v2,
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
  ///
  /// Handles the common pattern of resolving a property name from an evaluated
  /// expression:
  /// - `Expr::Ident` → symbol name as string
  /// - `Expr::Lit(Str)` → string value
  /// - `Expr::Lit(Num)` → number formatted as string
  /// - `Expr::Lit(BigInt)` → bigint formatted as string
  /// - All other variants → `None`
  ///
  /// `None` is an answer and not a fault -- a computed key or a value the
  /// language does not spell as a property name reaches here -- so the caller
  /// handles it rather than asserting it away.
  ///
  /// # Example
  /// ```ignore
  /// let Some(key) = property.as_string_key() else {
  ///   return None;
  /// };
  /// ```
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

/// Lets `stylex-css` read a marker out of an evaluated value without naming
/// any of the evaluator's types. Every accessor mirrors one of the type tests
/// the `when` functions perform on their second argument.
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

    // Two lazy passes rather than one collected `Vec`: inspecting a marker
    // object then costs no allocation. The second pass stops at the first
    // non-`$$css` key — the marker class — so for the two-key object
    // `defineMarker` emits, neither pass reads more than it must.
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

/// The plain key/value properties of an object literal, paired with their
/// values. Spreads, getters, setters, methods, shorthands and computed or
/// numeric keys are skipped, none of which a compiled StyleX object carries —
/// so what remains is exactly what `Object.keys` walks in the original.
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
///
/// Taken by reference rather than off a value, so a caller that holds only the
/// expression asks the same question without building a value to hold it --
/// which copied the whole subtree of a key that folded to an object or a list.
pub fn string_key_of_expr(expr: &Expr) -> Option<String> {
  match expr {
    // A property written as a name is that name. Read through the coercion
    // below instead, a name would answer for the global it spells -- `NaN`
    // and `Infinity` have a text and every other name has none -- where what
    // is asked for here is the property `obj.NaN` reads.
    Expr::Ident(ident) => Some(ident.sym.to_string()),
    // `ToPropertyKey`, which is `ToString` -- not Rust's `Display`. The two
    // part company on `-0` (`"0"` in the language, `"-0"` here) and on every
    // magnitude that takes exponential form (`"1e-7"` against `"0.0000001"`),
    // so `list[-0]` read no element and `obj[1e-7]` found no property.
    //
    // One coercion rather than a table of the kinds a key may be written as,
    // because a key is written and read in two places and they have to agree:
    // `{ [true]: 'red' }` names the property `true`, so `obj[true]` has to
    // look for that one. A table answered for a string and a number only, so
    // a boolean key was written nowhere and found nowhere.
    expr => to_js_string(expr),
  }
}

/// Reads a property name as a string, for the key shapes a compiled StyleX
/// object can carry.
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

  /// A property key is `ToPropertyKey`, which is `ToString`. Rust's `Display`
  /// agrees on the ordinary magnitudes and parts company on three shapes: it
  /// signs negative zero, and it never switches to exponential form at either
  /// end of the range.
  ///
  /// Only the first of those is reachable through a member read today -- an
  /// array reads `[-0]` as its first slot, where `"-0"` named no slot at all --
  /// but all three are the same one-line mistake, and the receiver that
  /// distinguishes the other two is one new fold away.
  #[test]
  fn a_number_key_is_spelled_the_way_the_language_spells_it() {
    assert_eq!(key_of(-0.0).as_deref(), Some("0"));
    assert_eq!(key_of(1e-7).as_deref(), Some("1e-7"));
    assert_eq!(key_of(1e21).as_deref(), Some("1e+21"));
  }

  /// And agrees with `Display` everywhere else, so the change is confined to the
  /// spellings above.
  #[test]
  fn an_ordinary_number_key_is_unchanged() {
    assert_eq!(key_of(0.0).as_deref(), Some("0"));
    assert_eq!(key_of(1.0).as_deref(), Some("1"));
    assert_eq!(key_of(42.5).as_deref(), Some("42.5"));
    assert_eq!(key_of(-7.0).as_deref(), Some("-7"));
  }
}
