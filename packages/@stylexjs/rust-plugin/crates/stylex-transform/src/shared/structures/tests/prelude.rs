//! Builders the pre-rule test files share.
//!
//! `RUST.md` permits a test prelude, because test code is not part of the
//! crate graph. The two files here build the same three values, so the
//! builders live in one place.

use crate::shared::structures::{
  null_pre_rule::NullPreRule,
  pre_rule::{PreRules, StylesPreRule},
  pre_rule_set::PreRuleSet,
};
use stylex_structures::pre_rule_value::PreRuleValue;

/// Builds a styles rule. `None` stands for a rule that the author wrote at the
/// top level of a namespace, where there is no key path.
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

/// A set of two or more rules. `create` collapses a shorter list, so a caller
/// asking for a set has to hand it at least two.
pub(crate) fn set(rules: Vec<PreRules>) -> PreRules {
  let created = PreRuleSet::create(rules);
  assert!(
    matches!(created, PreRules::PreRuleSet(_)),
    "expected a set, got {created:?}"
  );
  created
}
