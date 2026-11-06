//! Expanding one authored property into its declarations, and each validation mode's handling
//! of a value that cannot be expanded.

use std::borrow::Cow;

use log::Level;
use stylex_enums::property_validation_mode::PropertyValidationMode;
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
