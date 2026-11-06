//! What one compiled style value answers about itself.
//!
//! Every accessor here reads one variant and declines the rest, and a namespace
//! holds a mix of them: a class name, a `null` for a property the namespace
//! declares but does not set, a `true` for the compiled marker, an injectable
//! style, a variable pair. So each case asks one accessor for the variant it
//! reads *and* for a variant it does not, because an accessor that answered
//! `Some` for the wrong variant is what would let a `null` be written as a class
//! name.

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

/// The declarations one inline object holds.
fn object_value() -> FlatCompiledStyles {
  let mut values = FlatCompiledStyles::new();

  values.insert(
    "color".to_owned(),
    Rc::new(FlatCompiledStylesValue::String("blue".to_owned())),
  );

  values
}

/// Every variant, so a case that must name one it does not read has a list to
/// take it from rather than building one more.
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

/// How many variants answered `Some` to one accessor. Every accessor reads
/// exactly one variant, so the count is the whole assertion.
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

/// A tuple with no CSS type is the shape a plain variable takes, and the third
/// part is `None` rather than the accessor declining.
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

/// An empty class name is still a string: nothing about the accessor treats the
/// empty string as an absent one.
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

/// A number is its own variant, and the text spelling of the same number is
/// not it: `opacity: 0.5` and `opacity: '0.5'` are two declarations. Neither
/// of them reads back as the text a class name is.
#[test]
fn a_number_is_not_the_text_that_spells_it() {
  assert_ne!(FlatCompiledStylesValue::Number(0.5), string_value("0.5"));
  assert_eq!(FlatCompiledStylesValue::Number(0.5).as_class_name(), None);
  assert_eq!(FlatCompiledStylesValue::Number(2.0).as_class_name(), None);
}

/// An object holds the declarations it was given, and an object holding
/// nothing is an object and not an absent one -- a `props` call given `{}`
/// beside a compiled style reaches that shape.
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

/// The two zeroes are one value, so they are one hash. Their bit patterns
/// differ, which is what the reader folds away before it hashes them.
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

/// A list keeps the order it was written in, so two lists holding the same
/// elements in a different order are two values and two hashes.
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

/// Two values that are equal hash alike, which the derived spelling cannot
/// give a number.
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

/// No two variants hash alike while they hold the same thing, because the
/// variant itself is hashed first. A string `"true"` and a `true` are two
/// values a merge must tell apart.
#[test]
fn two_variants_holding_the_same_thing_hash_apart() {
  assert_ne!(
    hash_of(&string_value("true")),
    hash_of(&FlatCompiledStylesValue::Bool(true))
  );
}

/// An object is one value however its names were written down, so two that
/// hold the same declarations in either order hash alike. The map itself
/// compares that way, and a hash that read the names in order would make one
/// value into two.
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

/// Two objects of different shapes hash apart, even where the names and the
/// values they hold read out in the same order. The count of each object is
/// what separates them, and a merge that keys a cached style on the hash alone
/// would otherwise answer one style with another style's class names.
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

/// Every variant has a hash of its own, and no two of them share one. The
/// merge keys a cached style on this, so a variant left out of the reading
/// would answer with another style's class names.
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

/// The JSON a value spells, one kind at a time.
///
/// A `defineConsts` constant crosses a type that cannot hold this one, so the
/// kind travels as JSON. Every spelling below is what `JSON.stringify` writes
/// for the same value.
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

/// A whole number spells no fraction and the two zeroes spell the same `0`,
/// which is how JavaScript writes them. A number JSON has no word for is
/// spelled by name, so that a reader gets the number back rather than the
/// `null` that `JSON.stringify` writes.
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
  // Past the largest whole number a double holds exactly, the exponent form
  // is the reading left, as it is in the language.
  assert_eq!(
    FlatCompiledStylesValue::Number(1e21).to_json_text(),
    "1e+21"
  );
}

/// The reading is the inverse of the spelling, so a value crosses and comes
/// back as itself, including the four the spelling names rather than writes as
/// JSON.
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

  // `NaN` is never equal to itself, so it is read back and asked rather than
  // compared.
  assert_eq!(
    FlatCompiledStylesValue::from_json_text(
      &FlatCompiledStylesValue::Number(f64::NAN).to_json_text()
    )
    .as_number()
    .map(f64::is_nan),
    Some(true)
  );
}

/// Two readings do not survive, and neither is observable. A negative zero
/// comes back as a plain one, which is the same value and the same spelling in
/// the language. A number JSON has no word for, held inside an object, comes
/// back as `null` -- which is what `JSON.stringify` writes for it, and nothing
/// reads such a value back as a number.
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

/// Text that is not JSON reads as that text, which keeps a constant the
/// compiler cannot read from stopping a build over its spelling.
#[test]
fn reads_text_that_is_not_json_as_itself() {
  assert_eq!(
    FlatCompiledStylesValue::from_json_text("8px"),
    string_value("8px")
  );
}

/// The text JavaScript spells for each kind. Three readers ask it, and each
/// of them writes text.
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

/// A list spells its elements with a comma between, and a slot holding
/// nothing writes an empty run. A list of lists reads as one run, because
/// each of them spells its own text the same way.
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

/// A value kind that is no part of what an author writes spells no text.
#[test]
#[should_panic(expected = "Encountered a value kind that spells no text.")]
fn refuses_to_spell_a_kind_that_has_no_text() {
  FlatCompiledStylesValue::InjectableStyle(injectable()).to_js_text();
}

/// The same kind spells no JSON either.
#[test]
#[should_panic(expected = "Encountered a value kind that spells no JSON.")]
fn refuses_to_spell_a_kind_that_has_no_json() {
  FlatCompiledStylesValue::InjectableStyle(injectable()).to_json_text();
}

/// A number is its own kind, which is what the injected rule reads to decide
/// whether to write a number or text.
#[test]
fn a_number_answers_itself_and_nothing_else_does() {
  assert_eq!(FlatCompiledStylesValue::Number(0.5).as_number(), Some(0.5));
  assert_eq!(count_answering(|value| value.as_number().is_some()), 1);
}

/// The three questions `styleq` asks of a namespace entry. A class name is the
/// string, `null` marks a property the namespace clears, and `true` is the
/// compiled marker -- so only the string variant names a class.
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

/// One variant stands for a property that was given no value, and it is not
/// the `null` beside it.
///
/// The merge reads the two differently: a `null` clears the property and holds
/// it against every style behind it, while a property with no value is skipped
/// whole and left for a later style to set.
#[test]
fn one_value_stands_for_a_property_that_was_given_no_value() {
  assert!(FlatCompiledStylesValue::Undefined.is_undefined());
  assert!(!FlatCompiledStylesValue::Null.is_undefined());
  assert!(!FlatCompiledStylesValue::Undefined.is_null());
  assert_eq!(count_answering(StyleqValue::is_undefined), 1);
}

/// `false` is not the compiled marker. The marker is written as `true` and only
/// `true`, so a namespace carrying `$$css: false` is not treated as compiled.
#[test]
fn styleq_reads_the_compiled_marker_out_of_a_true_alone() {
  assert!(FlatCompiledStylesValue::Bool(true).is_true_bool());
  assert!(!FlatCompiledStylesValue::Bool(false).is_true_bool());
  assert_eq!(count_answering(StyleqValue::is_true_bool), 1);
}
