//! Builders the pre-rule test files share.

use crate::shared::structures::{
  null_pre_rule::NullPreRule,
  pre_rule::{PreRules, StylesPreRule},
  pre_rule_set::PreRuleSet,
};
use stylex_structures::pre_rule_value::PreRuleValue;

/// Builds a styles rule; `None` is a top-level namespace rule with no key path.
pub(crate) fn styles(property: &str, value: &str, key_path: Option<&[&str]>) -> StylesPreRule {
  StylesPreRule::new(
    property,
    PreRuleValue::string(value),
    key_path.map(|path| path.iter().map(|key| (*key).to_string()).collect()),
  )
}

/// The rule that stands for no rule.
pub(crate) fn null() -> PreRules {
  PreRules::NullPreRule(NullPreRule::new())
}

/// A set of two or more rules (`create` collapses a shorter list).
pub(crate) fn set(rules: Vec<PreRules>) -> PreRules {
  let created = PreRuleSet::create(rules);
  assert!(
    matches!(created, PreRules::PreRuleSet(_)),
    "expected a set, got {created:?}"
  );
  created
}
