//! The React properties and HTML attributes a merge is written back as.

use std::rc::Rc;

use stylex_state::{
  flat_compiled_styles_value::FlatCompiledStylesValue, types::FlatCompiledStyles,
};

use crate::shared::enums::data_structures::fn_result::FnResult;
use crate::shared::utils::core::tests::style_args::{
  ResultReader, inline, inline_unwritable, inline_value, style_of, styles,
};
use crate::shared::utils::core::{
  attrs::attrs,
  parse_nullable_style::{ResolvedArg, StyleObject},
  props::props,
  stylex::stylex,
};

fn values_of(result: FnResult) -> FlatCompiledStyles {
  match result.as_values() {
    Some(values) => values.clone(),
    None => panic!("the result holds no values: {result:?}"),
  }
}

fn text_of(values: &FlatCompiledStyles, key: &str) -> String {
  match values.get(key).map(Rc::as_ref) {
    Some(FlatCompiledStylesValue::String(text)) => text.clone(),
    other => panic!("the key {key} does not hold text: {other:?}"),
  }
}

fn style_pairs_of(values: &FlatCompiledStyles) -> Vec<(String, FlatCompiledStylesValue)> {
  match values.get("style").map(Rc::as_ref) {
    Some(FlatCompiledStylesValue::Object(declarations)) => declarations
      .iter()
      .map(|(key, value)| (key.clone(), value.as_ref().clone()))
      .collect(),
    other => panic!("the style is not an object of declarations: {other:?}"),
  }
}

fn text(value: &str) -> FlatCompiledStylesValue {
  FlatCompiledStylesValue::String(value.to_owned())
}

#[test]
fn names_the_merged_classes_as_one_class_name() {
  let values = values_of(props(&[style_of(&[("color", "xa"), ("margin", "xb")])]));

  assert_eq!(text_of(&values, "className"), "xa xb");
}

#[test]
fn writes_no_class_name_when_the_merge_wrote_no_class() {
  assert!(values_of(props(&[])).is_empty());
  assert!(values_of(props(&[ResolvedArg::style_object(StyleObject::Nullable)])).is_empty());
}

/// The runtime reads a style object, so `marginTop` keeps its name; the attribute half is
/// `writes_an_inline_style_out_as_css_text`.
#[test]
fn names_an_inline_style_as_the_pairs_the_author_wrote() {
  let values = values_of(props(&[styles(inline(&[("marginTop", "1px")]))]));

  assert_eq!(
    style_pairs_of(&values),
    [("marginTop".to_owned(), text("1px"))]
  );
}

/// Measured against `@stylexjs/babel-plugin` 0.19.0: `stylex.props({ opacity: 0.5 })` answers
/// `{ style: { opacity: 0.5 } }`, not the text `"0.5"`.
#[test]
fn keeps_the_kind_of_each_inline_value() {
  let opacity = values_of(props(&[styles(inline_value(
    "opacity",
    FlatCompiledStylesValue::Number(0.5),
  ))]));

  assert_eq!(
    style_pairs_of(&opacity),
    [("opacity".to_owned(), FlatCompiledStylesValue::Number(0.5))]
  );

  let color = values_of(props(&[styles(inline_value(
    "color",
    FlatCompiledStylesValue::Bool(true),
  ))]));

  assert_eq!(
    style_pairs_of(&color),
    [("color".to_owned(), FlatCompiledStylesValue::Bool(true))]
  );

  let hover = values_of(props(&[styles(inline_value(
    ":hover",
    FlatCompiledStylesValue::Object(inline(&[("color", "blue")])),
  ))]));

  assert_eq!(
    style_pairs_of(&hover),
    [(
      ":hover".to_owned(),
      FlatCompiledStylesValue::Object(inline(&[("color", "blue")]))
    )]
  );
}

/// The name keeps its first place. Measured against `@stylexjs/babel-plugin` 0.19.0:
/// `stylex.props({ color: 'red', opacity: 0.5 }, { opacity: true })` answers
/// `{ style: { color: "red", opacity: true } }`.
#[test]
fn a_later_declaration_of_another_kind_wins() {
  let mut earlier = inline(&[("color", "red")]);

  earlier.insert(
    "opacity".to_owned(),
    Rc::new(FlatCompiledStylesValue::Number(0.5)),
  );

  let values = values_of(props(&[
    styles(earlier),
    styles(inline_value("opacity", FlatCompiledStylesValue::Bool(true))),
  ]));

  assert_eq!(
    style_pairs_of(&values),
    [
      ("color".to_owned(), text("red")),
      ("opacity".to_owned(), FlatCompiledStylesValue::Bool(true)),
    ]
  );
}

/// The runtime dashes every capital in the attribute, custom properties included. Measured
/// against `@stylexjs/babel-plugin` 0.19.0: `stylex.attrs({ '--myColor': 'red' })` answers
/// `{ style: "--my-color:red" }`.
#[test]
fn spells_a_custom_property_the_way_each_result_spells_it() {
  let declaration = [styles(inline(&[("--myColor", "red")]))];

  assert_eq!(
    style_pairs_of(&values_of(props(&declaration))),
    [("--myColor".to_owned(), text("red"))]
  );
  assert_eq!(
    text_of(&values_of(attrs(&declaration)), "style"),
    "--my-color:red"
  );
}

/// Measured against `@stylexjs/babel-plugin` 0.19.0: `stylex.props({ color: ['red', 'blue'] })`
/// answers `{ style: { color: { "0": "red", "1": "blue" } } }`; as attributes,
/// `{ style: "color:red,blue" }`.
#[test]
fn writes_a_list_as_places_and_spells_it_as_a_join() {
  let list = FlatCompiledStylesValue::List(vec![
    Rc::new(text("red")),
    Rc::new(FlatCompiledStylesValue::Number(1.0)),
    Rc::new(FlatCompiledStylesValue::Null),
    Rc::new(FlatCompiledStylesValue::Undefined),
    Rc::new(FlatCompiledStylesValue::List(vec![Rc::new(text("a"))])),
  ]);

  assert_eq!(
    style_pairs_of(&values_of(props(&[styles(inline_value(
      "color",
      list.clone()
    ))]))),
    [("color".to_owned(), list.clone())]
  );

  assert_eq!(
    text_of(
      &values_of(attrs(&[styles(inline_value("color", list))])),
      "style"
    ),
    "color:red,1,,,a"
  );
}

#[test]
fn spells_a_list_holding_nothing_as_no_text() {
  let values = values_of(attrs(&[styles(inline_value(
    "color",
    FlatCompiledStylesValue::List(vec![]),
  ))]));

  assert_eq!(text_of(&values, "style"), "color:");
}

/// It does not clear an earlier style. Measured against `@stylexjs/babel-plugin` 0.19.0:
/// `stylex.props({ color: undefined })` answers `{}`, and
/// `stylex.props({ color: 'red' }, { color: undefined })` answers `{ style: { color: "red" } }`.
#[test]
fn skips_a_declaration_that_was_given_no_value() {
  let alone = values_of(props(&[styles(inline_value(
    "color",
    FlatCompiledStylesValue::Undefined,
  ))]));

  assert!(alone.is_empty());

  let after_a_value = values_of(props(&[
    styles(inline(&[("color", "red")])),
    styles(inline_value("color", FlatCompiledStylesValue::Undefined)),
  ]));

  assert_eq!(
    style_pairs_of(&after_a_value),
    [("color".to_owned(), text("red"))]
  );
}

/// The writers, not the merge, decide what such a value spells.
#[test]
fn carries_a_value_kind_an_inline_style_cannot_hold() {
  let values = values_of(props(&[styles(inline_unwritable("margin"))]));

  assert_eq!(style_pairs_of(&values).len(), 1);
}

#[test]
fn names_the_debug_source_only_when_there_is_one() {
  let mut style = FlatCompiledStyles::new();

  style.insert(
    stylex_constants::constants::common::COMPILED_KEY.to_owned(),
    Rc::new(FlatCompiledStylesValue::Bool(true)),
  );

  assert!(!values_of(props(&[styles(style)])).contains_key("data-style-src"));
}

#[test]
fn names_the_merged_classes_as_a_class_attribute() {
  let values = values_of(attrs(&[style_of(&[("color", "xa")])]));

  assert_eq!(text_of(&values, "class"), "xa");
  assert!(!values.contains_key("className"));
}

#[test]
fn writes_an_inline_style_out_as_css_text() {
  let values = values_of(attrs(&[styles(inline(&[
    ("marginTop", "1px"),
    ("color", "red"),
  ]))]));

  assert_eq!(text_of(&values, "style"), "margin-top:1px;color:red");
}

#[test]
fn writes_no_attribute_for_a_merge_that_wrote_nothing() {
  assert!(values_of(attrs(&[])).is_empty());
}

/// The `style` property half is `refuses_a_value_it_cannot_write`.
#[test]
#[should_panic(expected = "Encountered a value kind that spells no text.")]
fn refuses_an_inline_value_that_spells_no_text() {
  attrs(&[styles(inline_unwritable("margin"))]);
}

/// Measured against `@stylexjs/babel-plugin` 0.19.0: `stylex.attrs({ opacity: 0.5 })` answers
/// `{ style: "opacity:0.5" }`, and a nested object answers `":hover:[object Object]"`.
#[test]
fn spells_each_kind_of_inline_value_as_javascript_spells_it() {
  let mut style = inline(&[("color", "red")]);

  style.insert(
    "opacity".to_owned(),
    Rc::new(FlatCompiledStylesValue::Number(0.5)),
  );
  style.insert(
    "zIndex".to_owned(),
    Rc::new(FlatCompiledStylesValue::Number(1e21)),
  );
  style.insert(
    "flexGrow".to_owned(),
    Rc::new(FlatCompiledStylesValue::Bool(false)),
  );
  style.insert(
    ":hover".to_owned(),
    Rc::new(FlatCompiledStylesValue::Object(inline(&[(
      "color", "blue",
    )]))),
  );

  assert_eq!(
    text_of(&values_of(attrs(&[styles(style)])), "style"),
    "color:red;opacity:0.5;z-index:1e+21;flex-grow:false;:hover:[object Object]"
  );
}

/// A null inside an object is never read; the merge drops nulls only at the top level.
#[test]
fn spells_an_object_the_same_whatever_it_holds() {
  let holding_a_null = values_of(attrs(&[styles(inline_value(
    ":hover",
    FlatCompiledStylesValue::Object(inline_value("color", FlatCompiledStylesValue::Null)),
  ))]));

  let holding_nothing = values_of(attrs(&[styles(inline_value(
    ":hover",
    FlatCompiledStylesValue::Object(FlatCompiledStyles::new()),
  ))]));

  assert_eq!(text_of(&holding_a_null, "style"), ":hover:[object Object]");
  assert_eq!(text_of(&holding_nothing, "style"), ":hover:[object Object]");
}

/// As JavaScript spells it, not as emitter arithmetic.
#[test]
fn spells_a_number_that_has_no_digits_by_name() {
  let mut style = FlatCompiledStyles::new();

  style.insert(
    "opacity".to_owned(),
    Rc::new(FlatCompiledStylesValue::Number(f64::INFINITY)),
  );
  style.insert(
    "zIndex".to_owned(),
    Rc::new(FlatCompiledStylesValue::Number(f64::NEG_INFINITY)),
  );
  style.insert(
    "order".to_owned(),
    Rc::new(FlatCompiledStylesValue::Number(f64::NAN)),
  );

  assert_eq!(
    text_of(&values_of(attrs(&[styles(style)])), "style"),
    "opacity:Infinity;z-index:-Infinity;order:NaN"
  );
}

/// `props`/`attrs` answer values; `className` answers a string.
#[test]
fn each_call_answers_the_kind_it_makes() {
  let merged = [style_of(&[("color", "xa")])];

  assert!(props(&merged).as_values().is_some());
  assert!(attrs(&merged).as_values().is_some());
  assert!(stylex(&merged).as_class_name().is_some());
}
