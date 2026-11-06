use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use swc_core::{
  common::DUMMY_SP,
  ecma::ast::{Expr, Lit, Str},
};

use stylex_enums::{css_syntax::CSSSyntax, value_with_default::ValueWithDefault};
use stylex_structures::base_css_type::BaseCSSType;
use stylex_styleq::StyleqValue;
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
fn a_tuple_answers_its_three_parts_and_nothing_else_does() {
  let value = tuple_value();

  let Some((key, expr, base)) = value.as_tuple() else {
    panic!("a tuple did not answer its parts");
  };

  assert_eq!(key, "--x1abcdef");
  assert_eq!(
    expr
      .as_lit()
      .and_then(|lit| lit.as_str())
      .and_then(|str_lit| str_lit.value.as_str()),
    Some("red")
  );
  assert_eq!(base.as_ref(), Some(&css_type()));

  assert_eq!(count_answering(|value| value.as_tuple().is_some()), 1);
}

#[test]
fn a_tuple_without_a_css_type_still_answers_its_parts() {
  let value = FlatCompiledStylesValue::Tuple(
    "--x1abcdef".to_string(),
    Box::new(Expr::Lit(Lit::Str(Str {
      span: DUMMY_SP,
      value: "red".into(),
      raw: None,
    }))),
    None,
  );

  let Some((key, _, base)) = value.as_tuple() else {
    panic!("a tuple without a CSS type did not answer its parts");
  };

  assert_eq!(key, "--x1abcdef");
  assert_eq!(base.as_ref(), None);
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
fn an_empty_string_is_still_a_string() {
  assert_eq!(string_value("").as_string(), Some(&String::new()));
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
fn a_number_is_not_the_text_that_spells_it() {
  assert_ne!(FlatCompiledStylesValue::Number(0.5), string_value("0.5"));
  assert_eq!(FlatCompiledStylesValue::Number(0.5).as_class_name(), None);
  assert_eq!(FlatCompiledStylesValue::Number(2.0).as_class_name(), None);
}

#[test]
fn an_object_holds_the_declarations_it_was_given() {
  assert_eq!(
    FlatCompiledStylesValue::Object(object_value()),
    FlatCompiledStylesValue::Object(object_value())
  );
  assert_ne!(
    FlatCompiledStylesValue::Object(object_value()),
    FlatCompiledStylesValue::Object(FlatCompiledStyles::new())
  );
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
fn a_list_hashes_in_the_order_it_was_written() {
  let one_way = FlatCompiledStylesValue::List(vec![
    Rc::new(string_value("red")),
    Rc::new(string_value("blue")),
  ]);
  let the_other = FlatCompiledStylesValue::List(vec![
    Rc::new(string_value("blue")),
    Rc::new(string_value("red")),
  ]);

  assert_ne!(one_way, the_other);
  assert_ne!(hash_of(&one_way), hash_of(&the_other));
}

#[test]
fn a_value_hashes_by_what_it_holds() {
  assert_eq!(
    hash_of(&FlatCompiledStylesValue::Number(0.5)),
    hash_of(&FlatCompiledStylesValue::Number(0.5))
  );
  assert_ne!(
    hash_of(&FlatCompiledStylesValue::Number(0.5)),
    hash_of(&FlatCompiledStylesValue::Number(1.5))
  );
  assert_eq!(
    hash_of(&FlatCompiledStylesValue::Object(object_value())),
    hash_of(&FlatCompiledStylesValue::Object(object_value()))
  );
  assert_ne!(
    hash_of(&FlatCompiledStylesValue::Object(object_value())),
    hash_of(&FlatCompiledStylesValue::Object(FlatCompiledStyles::new()))
  );
}

#[test]
fn two_variants_holding_the_same_thing_hash_apart() {
  assert_ne!(
    hash_of(&string_value("true")),
    hash_of(&FlatCompiledStylesValue::Bool(true))
  );
}

#[test]
fn an_object_hashes_the_same_in_either_order() {
  let mut written_one_way = FlatCompiledStyles::new();

  written_one_way.insert(
    "color".to_owned(),
    Rc::new(FlatCompiledStylesValue::String("blue".to_owned())),
  );
  written_one_way.insert(
    "opacity".to_owned(),
    Rc::new(FlatCompiledStylesValue::Number(0.5)),
  );

  let mut written_the_other = FlatCompiledStyles::new();

  written_the_other.insert(
    "opacity".to_owned(),
    Rc::new(FlatCompiledStylesValue::Number(0.5)),
  );
  written_the_other.insert(
    "color".to_owned(),
    Rc::new(FlatCompiledStylesValue::String("blue".to_owned())),
  );

  assert_eq!(
    FlatCompiledStylesValue::Object(written_one_way.clone()),
    FlatCompiledStylesValue::Object(written_the_other.clone())
  );
  assert_eq!(
    hash_of(&FlatCompiledStylesValue::Object(written_one_way)),
    hash_of(&FlatCompiledStylesValue::Object(written_the_other))
  );
}

#[test]
fn two_objects_of_different_shapes_hash_apart() {
  let mut flat = FlatCompiledStyles::new();

  flat.insert(
    "x".to_owned(),
    Rc::new(FlatCompiledStylesValue::Object(FlatCompiledStyles::new())),
  );
  flat.insert("y".to_owned(), Rc::new(FlatCompiledStylesValue::Null));

  let mut deep_inner = FlatCompiledStyles::new();

  deep_inner.insert("y".to_owned(), Rc::new(FlatCompiledStylesValue::Null));

  let mut deep = FlatCompiledStyles::new();

  deep.insert(
    "x".to_owned(),
    Rc::new(FlatCompiledStylesValue::Object(deep_inner)),
  );

  assert_ne!(
    hash_of(&FlatCompiledStylesValue::Object(flat)),
    hash_of(&FlatCompiledStylesValue::Object(deep))
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
fn spells_the_json_of_each_kind() {
  assert_eq!(string_value("8px").to_json_text(), "\"8px\"");
  assert_eq!(FlatCompiledStylesValue::Number(800.0).to_json_text(), "800");
  assert_eq!(FlatCompiledStylesValue::Number(1.5).to_json_text(), "1.5");
  assert_eq!(FlatCompiledStylesValue::Bool(true).to_json_text(), "true");
  assert_eq!(FlatCompiledStylesValue::Bool(false).to_json_text(), "false");
  assert_eq!(FlatCompiledStylesValue::Null.to_json_text(), "null");
  assert_eq!(
    FlatCompiledStylesValue::Object(object_value()).to_json_text(),
    "{\"color\":\"blue\"}"
  );
  assert_eq!(
    FlatCompiledStylesValue::List(vec![Rc::new(string_value("x"))]).to_json_text(),
    "[\"x\"]"
  );
}

#[test]
fn spells_a_number_the_way_javascript_writes_it() {
  assert_eq!(FlatCompiledStylesValue::Number(-0.0).to_json_text(), "0");
  assert_eq!(FlatCompiledStylesValue::Number(-5.0).to_json_text(), "-5");
  assert_eq!(
    FlatCompiledStylesValue::Number(f64::NAN).to_json_text(),
    "NaN"
  );
  assert_eq!(
    FlatCompiledStylesValue::Number(f64::INFINITY).to_json_text(),
    "Infinity"
  );
  assert_eq!(
    FlatCompiledStylesValue::Number(f64::NEG_INFINITY).to_json_text(),
    "-Infinity"
  );
  assert_eq!(
    FlatCompiledStylesValue::Undefined.to_json_text(),
    "undefined"
  );

  assert_eq!(
    FlatCompiledStylesValue::Number(1e21).to_json_text(),
    "1e+21"
  );
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
fn names_the_two_readings_that_do_not_survive() {
  assert_eq!(
    FlatCompiledStylesValue::from_json_text(&FlatCompiledStylesValue::Number(-0.0).to_json_text()),
    FlatCompiledStylesValue::Number(0.0)
  );

  let nested = FlatCompiledStylesValue::Object(
    [(
      "a".to_owned(),
      Rc::new(FlatCompiledStylesValue::Number(f64::INFINITY)),
    )]
    .into_iter()
    .collect(),
  );

  assert_eq!(
    FlatCompiledStylesValue::from_json_text(&nested.to_json_text()),
    FlatCompiledStylesValue::Object(
      [("a".to_owned(), Rc::new(FlatCompiledStylesValue::Null))]
        .into_iter()
        .collect()
    )
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
fn spells_a_list_as_its_elements_with_commas() {
  let list = FlatCompiledStylesValue::List(vec![
    Rc::new(FlatCompiledStylesValue::Number(1.0)),
    Rc::new(FlatCompiledStylesValue::Null),
    Rc::new(FlatCompiledStylesValue::Undefined),
    Rc::new(FlatCompiledStylesValue::List(vec![Rc::new(string_value(
      "a",
    ))])),
  ]);

  assert_eq!(list.to_js_text(), "1,,,a");
  assert_eq!(FlatCompiledStylesValue::List(vec![]).to_js_text(), "");
}

#[test]
#[should_panic(expected = "Encountered a value kind that spells no text.")]
fn refuses_to_spell_a_kind_that_has_no_text() {
  FlatCompiledStylesValue::InjectableStyle(injectable()).to_js_text();
}

#[test]
#[should_panic(expected = "Encountered a value kind that spells no JSON.")]
fn refuses_to_spell_a_kind_that_has_no_json() {
  FlatCompiledStylesValue::InjectableStyle(injectable()).to_json_text();
}

#[test]
fn a_number_answers_itself_and_nothing_else_does() {
  assert_eq!(FlatCompiledStylesValue::Number(0.5).as_number(), Some(0.5));
  assert_eq!(count_answering(|value| value.as_number().is_some()), 1);
}

#[test]
fn styleq_reads_a_class_name_out_of_the_string_alone() {
  assert_eq!(string_value("x1e2nbdu").as_class_name(), Some("x1e2nbdu"));
  assert_eq!(count_answering(|value| value.as_class_name().is_some()), 1);
}

#[test]
fn styleq_reads_null_out_of_the_null_alone() {
  assert!(FlatCompiledStylesValue::Null.is_null());
  assert_eq!(count_answering(StyleqValue::is_null), 1);
}

#[test]
fn one_value_stands_for_a_property_that_was_given_no_value() {
  assert!(FlatCompiledStylesValue::Undefined.is_undefined());
  assert!(!FlatCompiledStylesValue::Null.is_undefined());
  assert!(!FlatCompiledStylesValue::Undefined.is_null());
  assert_eq!(count_answering(StyleqValue::is_undefined), 1);
}

#[test]
fn styleq_reads_the_compiled_marker_out_of_a_true_alone() {
  assert!(FlatCompiledStylesValue::Bool(true).is_true_bool());
  assert!(!FlatCompiledStylesValue::Bool(false).is_true_bool());
  assert_eq!(count_answering(StyleqValue::is_true_bool), 1);
}
