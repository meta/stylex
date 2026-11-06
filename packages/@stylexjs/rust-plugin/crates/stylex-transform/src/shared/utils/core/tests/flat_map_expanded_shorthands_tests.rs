//! Expanding one authored property into its declarations, and each validation mode's handling
//! of a value that cannot be expanded.

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

fn keys_of(pairs: &[OrderPair]) -> Vec<String> {
  pairs.iter().map(|pair| pair.0.to_string()).collect()
}

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

/// Emptying the reached properties makes a later declaration unset rather than shadowed.
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

#[test]
fn a_variable_reference_key_becomes_the_variable_it_names() {
  let pairs = expand(
    "var(--color)",
    PreRuleValue::string("red"),
    PropertyValidationMode::Throw,
  );

  assert_eq!(keys_of(&pairs), ["--color"]);
}

/// Only a whole reference is unwrapped; a key merely starting with `var(` is a property name.
#[test]
fn a_key_that_is_not_a_whole_reference_keeps_its_name() {
  let pairs = expand(
    "var(--color",
    PreRuleValue::string("red"),
    PropertyValidationMode::Throw,
  );

  assert_eq!(keys_of(&pairs), ["var(--color"]);
}

/// The property that reads the number decides the unit.
#[test]
fn a_number_stays_a_number() {
  let pairs = expand(
    "width",
    PreRuleValue::number(1.0),
    PropertyValidationMode::Throw,
  );

  assert_eq!(pairs[0].1, Some(TRawValue::Number(1.0)));
}

#[test]
fn a_numeric_expression_stays_a_number() {
  let pairs = expand(
    "width",
    PreRuleValue::Expr(expr("1")),
    PropertyValidationMode::Throw,
  );

  assert_eq!(pairs[0].1, Some(TRawValue::Number(1.0)));
}

#[test]
fn a_string_expression_becomes_its_text() {
  let pairs = expand(
    "color",
    PreRuleValue::Expr(expr("'red'")),
    PropertyValidationMode::Throw,
  );

  assert_eq!(pairs[0].1, Some(TRawValue::String("red".to_owned())));
}

/// JavaScript spells a big integer as its digits.
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

/// The property survives carrying the absence.
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

/// A boolean only reaches here from calls with no value validator in front.
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

/// Unusable, not absent: dropping it silently would lose a declaration.
#[test]
#[should_panic(expected = "Failed to convert literal value to string in shorthand expansion.")]
fn refuses_a_literal_that_spells_no_text() {
  expand(
    "color",
    PreRuleValue::Expr(expr("/re/")),
    PropertyValidationMode::Throw,
  );
}

/// Each longhand would need its own fallback list.
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

/// The expansion has to read the value to split it across properties.
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

/// The expansion refuses `all` by name with its own message.
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
