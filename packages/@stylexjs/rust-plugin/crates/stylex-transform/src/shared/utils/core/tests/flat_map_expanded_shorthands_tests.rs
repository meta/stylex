//! Tests for the expansion of one authored property into the declarations it
//! stands for, and for what each validation mode does with a value that cannot
//! be expanded.

use std::borrow::Cow;

use log::Level;
use stylex_enums::{
  property_validation_mode::PropertyValidationMode, style_resolution::StyleResolution,
};
use stylex_structures::{
  order_pair::OrderPair, pre_rule_value::PreRuleValue, raw_value::TRawValue,
  stylex_state_options::StyleXStateOptions,
};

use crate::shared::utils::core::flat_map_expanded_shorthands::flat_map_expanded_shorthands;
use crate::tests::capturing_logger::logged_at;
use crate::tests::support::expr;

fn options_with(mode: PropertyValidationMode) -> StyleXStateOptions {
  let mut options = StyleXStateOptions::default();

  options.core.property_validation_mode = mode;

  options
}

fn expand(key: &str, value: PreRuleValue, mode: PropertyValidationMode) -> Vec<OrderPair> {
  flat_map_expanded_shorthands((Cow::Borrowed(key), value), &options_with(mode))
}

/// The keys of the answer, in the order the expansion gives them.
fn keys_of(pairs: &[OrderPair]) -> Vec<String> {
  pairs.iter().map(|pair| pair.0.to_string()).collect()
}

/// A property that stands for itself is one pair, and the value reaches it
/// unchanged.
#[test]
fn a_plain_property_expands_to_itself() {
  let pairs = expand(
    "color",
    PreRuleValue::string("red"),
    PropertyValidationMode::Throw,
  );

  assert_eq!(keys_of(&pairs), ["color"]);
  assert_eq!(pairs[0].1, Some(TRawValue::String("red".to_owned())));
}

/// A shorthand carries its value and empties every property it reaches, so a
/// later declaration of one of them is unset rather than shadowed.
#[test]
fn a_shorthand_expands_to_the_properties_it_reaches() {
  let pairs = flat_map_expanded_shorthands(
    (Cow::Borrowed("margin"), PreRuleValue::string("10px")),
    &StyleXStateOptions::default().with_style_resolution(StyleResolution::ApplicationOrder),
  );

  assert_eq!(pairs[0].0, "margin");
  assert_eq!(pairs[0].1, Some(TRawValue::String("10px".to_owned())));
  assert!(keys_of(&pairs).contains(&"marginTop".to_owned()));
  assert!(
    pairs[1..].iter().all(|pair| pair.1.is_none()),
    "a property the shorthand reaches carries a value"
  );
}

/// The resolution decides which table the key is read from, so the same
/// shorthand expands to a different set under each.
#[test]
fn the_resolution_decides_the_expansion() {
  let entry = || (Cow::Borrowed("margin"), PreRuleValue::string("10px"));

  let application = flat_map_expanded_shorthands(
    entry(),
    &StyleXStateOptions::default().with_style_resolution(StyleResolution::ApplicationOrder),
  );
  let legacy = flat_map_expanded_shorthands(
    entry(),
    &StyleXStateOptions::default().with_style_resolution(StyleResolution::LegacyExpandShorthands),
  );
  let specificity = flat_map_expanded_shorthands(
    entry(),
    &StyleXStateOptions::default().with_style_resolution(StyleResolution::PropertySpecificity),
  );

  assert_ne!(keys_of(&application), keys_of(&legacy));
  assert!(!keys_of(&specificity).is_empty());
}

/// A key spelled as a CSS variable reference is written under the variable it
/// names, because that is the declaration the author meant.
#[test]
fn a_variable_reference_key_becomes_the_variable_it_names() {
  let pairs = expand(
    "var(--color)",
    PreRuleValue::string("red"),
    PropertyValidationMode::Throw,
  );

  assert_eq!(keys_of(&pairs), ["--color"]);
}

/// Only a whole reference is unwrapped. A key that merely starts with `var(`
/// is a property name, and renaming it would lose the declaration.
#[test]
fn a_key_that_is_not_a_whole_reference_keeps_its_name() {
  let pairs = expand(
    "var(--color",
    PreRuleValue::string("red"),
    PropertyValidationMode::Throw,
  );

  assert_eq!(keys_of(&pairs), ["var(--color"]);
}

/// A number stays a number, so the property that reads it decides the unit.
#[test]
fn a_number_stays_a_number() {
  let pairs = expand(
    "width",
    PreRuleValue::number(1.0),
    PropertyValidationMode::Throw,
  );

  assert_eq!(pairs[0].1, Some(TRawValue::Number(1.0)));
}

/// A numeric expression is read as a number for the same reason.
#[test]
fn a_numeric_expression_stays_a_number() {
  let pairs = expand(
    "width",
    PreRuleValue::Expr(expr("1")),
    PropertyValidationMode::Throw,
  );

  assert_eq!(pairs[0].1, Some(TRawValue::Number(1.0)));
}

/// A string expression is read as the text it spells.
#[test]
fn a_string_expression_becomes_its_text() {
  let pairs = expand(
    "color",
    PreRuleValue::Expr(expr("'red'")),
    PropertyValidationMode::Throw,
  );

  assert_eq!(pairs[0].1, Some(TRawValue::String("red".to_owned())));
}

/// A big integer is read as its digits, which is what JavaScript spells it as.
#[test]
fn a_big_integer_expression_becomes_its_digits() {
  let pairs = expand(
    "zIndex",
    PreRuleValue::Expr(expr("9007199254740993n")),
    PropertyValidationMode::Throw,
  );

  assert_eq!(
    pairs[0].1,
    Some(TRawValue::String("9007199254740993".to_owned()))
  );
}

/// `null` declares nothing, and the property survives carrying that absence.
#[test]
fn a_null_value_declares_nothing() {
  assert_eq!(
    expand("color", PreRuleValue::Null, PropertyValidationMode::Throw)[0].1,
    None
  );
  assert_eq!(
    expand(
      "color",
      PreRuleValue::Expr(expr("null")),
      PropertyValidationMode::Throw
    )[0]
      .1,
    None
  );
}

/// A boolean reaches here only from the calls that have no value validator in
/// front of them, and it declares nothing either.
#[test]
fn a_boolean_value_declares_nothing() {
  assert_eq!(
    expand(
      "color",
      PreRuleValue::Expr(expr("true")),
      PropertyValidationMode::Throw
    )[0]
      .1,
    None
  );
}

/// A regular expression is not an absent value, it is an unusable one. Saying
/// nothing about it would drop a declaration the author meant to write.
#[test]
#[should_panic(expected = "Failed to convert literal value to string in shorthand expansion.")]
fn refuses_a_literal_that_spells_no_text() {
  expand(
    "color",
    PreRuleValue::Expr(expr("/re/")),
    PropertyValidationMode::Throw,
  );
}

/// A shorthand has no place to put a fallback list, because each of its
/// properties would need one.
#[test]
#[should_panic(expected = "Cannot use fallbacks for shorthands. Use the expansion instead.")]
fn refuses_a_fallback_list_when_it_must_throw() {
  expand(
    "margin",
    PreRuleValue::Vec(vec![TRawValue::String("10px".to_owned())]),
    PropertyValidationMode::Throw,
  );
}

#[test]
fn reports_a_fallback_list_when_it_must_warn() {
  let mut pairs = Vec::new();

  let messages = logged_at(Level::Warn, || {
    pairs = expand(
      "margin",
      PreRuleValue::Vec(vec![TRawValue::String("10px".to_owned())]),
      PropertyValidationMode::Warn,
    );
  });

  assert!(pairs.is_empty());
  assert_eq!(
    messages,
    ["Cannot use fallbacks for shorthands. Use the expansion instead."]
  );
}

#[test]
fn says_nothing_about_a_fallback_list_when_it_must_stay_silent() {
  let messages = logged_at(Level::Warn, || {
    assert!(
      expand(
        "margin",
        PreRuleValue::Vec(vec![TRawValue::String("10px".to_owned())]),
        PropertyValidationMode::Silent,
      )
      .is_empty()
    );
  });

  assert!(messages.is_empty());
}

/// An expression that folded to no literal cannot be expanded, because the
/// expansion has to read the value to give each property its part of it.
#[test]
#[should_panic(expected = "Cannot use expressions for shorthands. Use the expansion instead.")]
fn refuses_an_unfolded_expression_when_it_must_throw() {
  expand(
    "margin",
    PreRuleValue::Expr(expr("a + b")),
    PropertyValidationMode::Throw,
  );
}

#[test]
fn reports_an_unfolded_expression_when_it_must_warn() {
  let mut pairs = Vec::new();

  let messages = logged_at(Level::Warn, || {
    pairs = expand(
      "margin",
      PreRuleValue::Expr(expr("a + b")),
      PropertyValidationMode::Warn,
    );
  });

  assert!(pairs.is_empty());
  assert_eq!(
    messages,
    ["Cannot use expressions for shorthands. Use the expansion instead."]
  );
}

#[test]
fn says_nothing_about_an_unfolded_expression_when_it_must_stay_silent() {
  let messages = logged_at(Level::Warn, || {
    assert!(
      expand(
        "margin",
        PreRuleValue::Expr(expr("a + b")),
        PropertyValidationMode::Silent,
      )
      .is_empty()
    );
  });

  assert!(messages.is_empty());
}

/// `all` is a shorthand the expansion refuses by name, so it carries the
/// message the expansion itself wrote rather than one of the two above.
#[test]
#[should_panic(expected = "all is not supported")]
fn refuses_an_unsupported_shorthand_when_it_must_throw() {
  expand(
    "all",
    PreRuleValue::string("unset"),
    PropertyValidationMode::Throw,
  );
}

#[test]
fn reports_an_unsupported_shorthand_when_it_must_warn() {
  let mut pairs = Vec::new();

  let messages = logged_at(Level::Warn, || {
    pairs = expand(
      "all",
      PreRuleValue::string("unset"),
      PropertyValidationMode::Warn,
    );
  });

  assert!(pairs.is_empty());
  assert_eq!(messages, ["all is not supported"]);
}

#[test]
fn says_nothing_about_an_unsupported_shorthand_when_it_must_stay_silent() {
  let messages = logged_at(Level::Warn, || {
    assert!(
      expand(
        "all",
        PreRuleValue::string("unset"),
        PropertyValidationMode::Silent,
      )
      .is_empty()
    );
  });

  assert!(messages.is_empty());
}
