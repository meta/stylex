//! The React properties and HTML attributes a merge is written back as.

use std::rc::Rc;

use stylex_state::{
  flat_compiled_styles_value::FlatCompiledStylesValue, types::FlatCompiledStyles,
};

use crate::shared::enums::data_structures::fn_result::FnResult;
use crate::shared::utils::core::tests::style_args::{
  ResultReader, inline, inline_unwritable, inline_value, styles,
};
use crate::shared::utils::core::{attrs::attrs, props::props};

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

/// The `style` property half is `refuses_a_value_it_cannot_write`.
#[test]
#[should_panic(expected = "Encountered a value kind that spells no text.")]
fn refuses_an_inline_value_that_spells_no_text() {
  attrs(&[styles(inline_unwritable("margin"))]);
}
