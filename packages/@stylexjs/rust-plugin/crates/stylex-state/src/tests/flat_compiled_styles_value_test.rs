use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use swc_core::{
  common::DUMMY_SP,
  ecma::ast::{Expr, Lit, Str},
};

use stylex_enums::{css_syntax::CSSSyntax, value_with_default::ValueWithDefault};
use stylex_structures::base_css_type::BaseCSSType;
use stylex_types::structures::injectable_style::InjectableStyle;

use crate::{flat_compiled_styles_value::FlatCompiledStylesValue, types::FlatCompiledStyles};

fn string_value(value: &str) -> FlatCompiledStylesValue {
  FlatCompiledStylesValue::String(value.to_string())
}

fn injectable() -> InjectableStyle {
  InjectableStyle {
    ltr: ".x1e2nbdu{color:red}".to_string(),
    rtl: None,
    priority: Some(3000.0),
  }
}

fn css_type() -> BaseCSSType {
  BaseCSSType {
    value: ValueWithDefault::String("red".to_string()),
    syntax: CSSSyntax::Color,
  }
}

fn tuple_value() -> FlatCompiledStylesValue {
  FlatCompiledStylesValue::Tuple(
    "--x1abcdef".to_string(),
    Box::new(Expr::Lit(Lit::Str(Str {
      span: DUMMY_SP,
      value: "red".into(),
      raw: None,
    }))),
    Some(css_type()),
  )
}

fn object_value() -> FlatCompiledStyles {
  let mut values = FlatCompiledStyles::new();

  values.insert(
    "color".to_owned(),
    Rc::new(FlatCompiledStylesValue::String("blue".to_owned())),
  );

  values
}

fn every_variant() -> Vec<FlatCompiledStylesValue> {
  vec![
    string_value("x1e2nbdu"),
    FlatCompiledStylesValue::Number(0.5),
    FlatCompiledStylesValue::Object(object_value()),
    FlatCompiledStylesValue::List(vec![Rc::new(string_value("blue"))]),
    FlatCompiledStylesValue::Null,
    FlatCompiledStylesValue::Undefined,
    FlatCompiledStylesValue::InjectableStyle(injectable()),
    FlatCompiledStylesValue::Bool(true),
    tuple_value(),
  ]
}

fn count_answering(accessor: impl Fn(&FlatCompiledStylesValue) -> bool) -> usize {
  every_variant()
    .iter()
    .filter(|value| accessor(value))
    .count()
}

#[test]
fn a_string_answers_itself_and_nothing_else_does() {
  assert_eq!(
    string_value("x1e2nbdu").as_string(),
    Some(&"x1e2nbdu".to_string())
  );
  assert_eq!(count_answering(|value| value.as_string().is_some()), 1);
}

#[test]
fn an_injectable_style_answers_itself_and_nothing_else_does() {
  let value = FlatCompiledStylesValue::InjectableStyle(injectable());

  assert_eq!(value.as_injectable_style(), Some(&injectable()));
  assert_eq!(
    count_answering(|value| value.as_injectable_style().is_some()),
    1
  );
}

#[test]
fn a_bool_answers_itself_and_nothing_else_does() {
  assert_eq!(FlatCompiledStylesValue::Bool(true).as_bool(), Some(&true));
  assert_eq!(FlatCompiledStylesValue::Bool(false).as_bool(), Some(&false));
  assert_eq!(count_answering(|value| value.as_bool().is_some()), 1);
}

#[test]
fn a_null_answers_itself_and_nothing_else_does() {
  assert_eq!(FlatCompiledStylesValue::Null.as_null(), Some(()));
  assert_eq!(count_answering(|value| value.as_null().is_some()), 1);
}

#[test]
fn the_two_zeroes_are_one_value_and_one_hash() {
  assert_eq!(
    FlatCompiledStylesValue::Number(-0.0),
    FlatCompiledStylesValue::Number(0.0)
  );
  assert_eq!(
    hash_of(&FlatCompiledStylesValue::Number(-0.0)),
    hash_of(&FlatCompiledStylesValue::Number(0.0))
  );
}

#[test]
fn every_variant_hashes_apart_from_every_other() {
  let mut hashes = every_variant().iter().map(hash_of).collect::<Vec<_>>();
  let count = hashes.len();

  hashes.sort_unstable();
  hashes.dedup();

  assert_eq!(hashes.len(), count);
}

fn hash_of(value: &FlatCompiledStylesValue) -> u64 {
  let mut hasher = DefaultHasher::new();

  value.hash(&mut hasher);

  hasher.finish()
}

#[test]
fn reads_back_every_kind_it_spells() {
  for value in [
    string_value("8px"),
    FlatCompiledStylesValue::Number(800.0),
    FlatCompiledStylesValue::Number(1.5),
    FlatCompiledStylesValue::Number(f64::INFINITY),
    FlatCompiledStylesValue::Number(f64::NEG_INFINITY),
    FlatCompiledStylesValue::Bool(true),
    FlatCompiledStylesValue::Null,
    FlatCompiledStylesValue::Undefined,
    FlatCompiledStylesValue::Object(object_value()),
    FlatCompiledStylesValue::List(vec![Rc::new(string_value("x"))]),
  ] {
    assert_eq!(
      FlatCompiledStylesValue::from_json_text(&value.to_json_text()),
      value
    );
  }

  assert_eq!(
    FlatCompiledStylesValue::from_json_text(
      &FlatCompiledStylesValue::Number(f64::NAN).to_json_text()
    )
    .as_number()
    .map(f64::is_nan),
    Some(true)
  );
}

#[test]
fn reads_text_that_is_not_json_as_itself() {
  assert_eq!(
    FlatCompiledStylesValue::from_json_text("8px"),
    string_value("8px")
  );
}

#[test]
fn spells_the_text_javascript_writes_for_each_kind() {
  assert_eq!(string_value("8px").to_js_text(), "8px");
  assert_eq!(FlatCompiledStylesValue::Number(0.5).to_js_text(), "0.5");
  assert_eq!(FlatCompiledStylesValue::Number(1e21).to_js_text(), "1e+21");
  assert_eq!(FlatCompiledStylesValue::Bool(true).to_js_text(), "true");
  assert_eq!(FlatCompiledStylesValue::Bool(false).to_js_text(), "false");
  assert_eq!(FlatCompiledStylesValue::Null.to_js_text(), "null");
  assert_eq!(FlatCompiledStylesValue::Undefined.to_js_text(), "undefined");
  assert_eq!(
    FlatCompiledStylesValue::Object(object_value()).to_js_text(),
    "[object Object]"
  );
}

#[test]
#[should_panic(expected = "Encountered a value kind that spells no JSON.")]
fn refuses_to_spell_a_kind_that_has_no_json() {
  FlatCompiledStylesValue::InjectableStyle(injectable()).to_json_text();
}
