//! Tests for the CSS a variable group is written out as.

use indexmap::IndexMap;
use stylex_enums::{css_syntax::CSSSyntax, value_with_default::ValueWithDefault};
use stylex_state::{
  flat_compiled_styles_value::FlatCompiledStylesValue,
  types::{ClassPathsInNamespace, InjectableStylesMap},
};
use stylex_structures::base_css_type::BaseCSSType;
use stylex_types::{
  enums::data_structures::injectable_style::InjectableStyleKind,
  structures::injectable_style::InjectableStyle,
};

use crate::shared::utils::core::define_vars_utils::collect_vars_by_at_rules;
use crate::tests::support::expr;

/// A variable whose value is the expression `code` spells.
fn variable(code: &str) -> FlatCompiledStylesValue {
  FlatCompiledStylesValue::Tuple("xhash".to_owned(), Box::new(expr(code)), None)
}

/// A typed variable, whose declared syntax carries the value `code` spells.
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

#[test]
fn writes_a_plain_value_under_the_default_at_rule() {
  let collection = collect(&variable("'8px'"));

  assert_eq!(collection["default"], ["--xhash:8px;"]);
}

#[test]
fn writes_a_number_the_way_javascript_spells_it() {
  assert_eq!(collect(&variable("8"))["default"], ["--xhash:8;"]);
  assert_eq!(collect(&variable("1e21"))["default"], ["--xhash:1e+21;"]);
}

/// A value written as absent declares nothing, so the group carries no entry
/// for it at all.
#[test]
fn writes_nothing_for_an_absent_value() {
  assert!(collect(&variable("null")).is_empty());
}

/// An object gives one value per at-rule, and the at-rules are kept in the name
/// of the key they are written under.
#[test]
fn writes_one_value_for_each_at_rule_an_object_names() {
  let collection = collect(&variable("{ default: '8px', '@media print': '0px' }"));

  assert_eq!(collection["default"], ["--xhash:8px;"]);
  assert_eq!(collection["@media print"], ["--xhash:0px;"]);
}

/// Nested at-rules are joined under one key, sorted, so the same two rules
/// written in either order name one place.
#[test]
fn joins_nested_at_rules_into_one_key() {
  let collection = collect(&variable(
    "{ default: '8px', '@media print': { default: '0px', '@supports (a:b)': '1px' } }",
  ));

  assert!(
    collection
      .keys()
      .any(|key| key.contains("@media print") && key.contains("@supports (a:b)")),
    "the nested rules are not joined: {:?}",
    collection.keys().collect::<Vec<_>>()
  );
}

/// An object of at-rules with no `default` leaves the variable undeclared
/// outside those rules, which is a fault the author is told about by name.
#[test]
#[should_panic(expected = "cornerRadius")]
fn refuses_an_object_that_names_no_default() {
  collect(&variable("{ '@media print': '0px' }"));
}

/// A variable takes one value. An array spells several, and there is no rule to
/// pick one of them.
#[test]
#[should_panic(expected = "Array values are not supported in defineVars()")]
fn refuses_an_array_value() {
  collect(&variable("['8px', '0px']"));
}

/// A literal that spells no text cannot be written into a declaration.
#[test]
#[should_panic(expected = "Expected a CSS custom property (variable) reference.")]
fn refuses_a_literal_that_spells_no_text() {
  collect(&variable("/re/"));
}

/// Only a variable and its value can be collected. Anything else means the
/// group was not compiled.
#[test]
#[should_panic(expected = "The values argument must be a plain object.")]
fn refuses_a_value_that_is_not_a_variable() {
  collect(&FlatCompiledStylesValue::Bool(true));
}

/// A value the compiler could not fold is left out, because there is nothing to
/// write for it.
#[test]
fn writes_nothing_for_a_value_that_did_not_fold() {
  assert!(collect(&variable("a + b")).is_empty());
}

/// A typed variable records the syntax it was declared with, and the value the
/// syntax falls back to.
#[test]
fn records_the_declared_syntax_of_a_typed_variable() {
  let mut typed_variables = InjectableStylesMap::new();

  let mut syntax_value: IndexMap<String, ValueWithDefault> = IndexMap::new();

  syntax_value.insert(
    "default".to_owned(),
    ValueWithDefault::String("blue".to_owned()),
  );

  collect_vars_by_at_rules(
    &"colour".to_owned(),
    &typed_variable("'red'", ValueWithDefault::Map(syntax_value)),
    &mut ClassPathsInNamespace::new(),
    &[],
    &mut typed_variables,
  );

  assert_eq!(
    typed_variables["xhash"].as_ref(),
    &InjectableStyleKind::Regular(InjectableStyle {
      ltr: "@property --xhash { syntax: \"<color>\"; inherits: true; initial-value: blue }"
        .to_owned(),
      rtl: None,
      priority: Some(0.0),
    })
  );
}

/// A nested syntax value is read down to the value the innermost `default`
/// names.
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

/// A declared syntax with no `default` has no value to fall back to, which
/// leaves the variable undefined until something sets it.
#[test]
#[should_panic(expected = "CSS type requires a default value but none was provided.")]
fn refuses_a_declared_syntax_that_names_no_default() {
  collect(&typed_variable(
    "'red'",
    ValueWithDefault::Map(IndexMap::new()),
  ));
}

/// A syntax whose value is not a set of named values has no `default` to read
/// at all.
#[test]
#[should_panic(expected = "Value must be a map")]
fn refuses_a_declared_syntax_that_names_no_values() {
  collect(&typed_variable(
    "'red'",
    ValueWithDefault::String("red".to_owned()),
  ));
}
