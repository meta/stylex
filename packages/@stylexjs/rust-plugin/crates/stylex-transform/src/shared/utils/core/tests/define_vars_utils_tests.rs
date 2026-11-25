//! The CSS a variable group is written out as.

use indexmap::IndexMap;
use stylex_enums::{css_syntax::CSSSyntax, value_with_default::ValueWithDefault};
use stylex_state::{
  flat_compiled_styles_value::FlatCompiledStylesValue,
  types::{ClassPathsInNamespace, InjectableStylesMap},
};
use stylex_structures::base_css_type::BaseCSSType;
use stylex_types::enums::data_structures::injectable_style::InjectableStyleKind;

use crate::shared::utils::core::define_vars_utils::collect_vars_by_at_rules;
use crate::tests::support::expr;

fn variable(code: &str) -> FlatCompiledStylesValue {
  FlatCompiledStylesValue::Tuple("xhash".to_owned(), Box::new(expr(code)), None)
}

fn typed_variable(code: &str, value: ValueWithDefault) -> FlatCompiledStylesValue {
  FlatCompiledStylesValue::Tuple(
    "xhash".to_owned(),
    Box::new(expr(code)),
    Some(BaseCSSType {
      value,
      syntax: CSSSyntax::Color,
    }),
  )
}

/// The declarations the value writes, keyed by the at-rule they belong under.
fn collect(value: &FlatCompiledStylesValue) -> ClassPathsInNamespace {
  let mut collection = ClassPathsInNamespace::new();

  collect_vars_by_at_rules(
    &"cornerRadius".to_owned(),
    value,
    &mut collection,
    &[],
    &mut InjectableStylesMap::new(),
  );

  collection
}

/// The author is told about the missing `default` by name.
#[test]
#[should_panic(expected = "cornerRadius")]
fn refuses_an_object_that_names_no_default() {
  collect(&variable("{ '@media print': '0px' }"));
}

#[test]
#[should_panic(expected = "Expected a CSS custom property (variable) reference.")]
fn refuses_a_literal_that_spells_no_text() {
  collect(&variable("/re/"));
}

#[test]
#[should_panic(expected = "The values argument must be a plain object.")]
fn refuses_a_value_that_is_not_a_variable() {
  collect(&FlatCompiledStylesValue::Bool(true));
}

#[test]
fn writes_nothing_for_a_value_that_did_not_fold() {
  assert!(collect(&variable("a + b")).is_empty());
}

#[test]
fn reads_the_initial_value_through_a_nested_default() {
  let mut inner: IndexMap<String, ValueWithDefault> = IndexMap::new();

  inner.insert("default".to_owned(), ValueWithDefault::Number(8.0));

  let mut outer: IndexMap<String, ValueWithDefault> = IndexMap::new();

  outer.insert("default".to_owned(), ValueWithDefault::Map(inner));

  let mut typed_variables = InjectableStylesMap::new();

  collect_vars_by_at_rules(
    &"colour".to_owned(),
    &typed_variable("'red'", ValueWithDefault::Map(outer)),
    &mut ClassPathsInNamespace::new(),
    &[],
    &mut typed_variables,
  );

  assert!(match typed_variables["xhash"].as_ref() {
    InjectableStyleKind::Regular(style) => style.ltr.ends_with("initial-value: 8 }"),
    other => panic!("the variable records no declared syntax: {other:?}"),
  });
}

#[test]
#[should_panic(expected = "CSS type requires a default value but none was provided.")]
fn refuses_a_declared_syntax_that_names_no_default() {
  collect(&typed_variable(
    "'red'",
    ValueWithDefault::Map(IndexMap::new()),
  ));
}

#[test]
#[should_panic(expected = "Value must be a map")]
fn refuses_a_declared_syntax_that_names_no_values() {
  collect(&typed_variable(
    "'red'",
    ValueWithDefault::String("red".to_owned()),
  ));
}
