use stylex_macros::stylex_panic;

use crate::shared::structures::pre_rule::{CompiledResult, ComputedStyle};
use stylex_constants::constants::messages::RULE_SET_EMPTY;
use stylex_state::state_manager::StateManager;
use stylex_structures::pre_rule_value::PreRuleValue;

use super::{
  null_pre_rule::NullPreRule,
  pre_rule::{PreRule, PreRules},
};
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PreRuleSet {
  rules: Vec<PreRules>,
}

impl PreRuleSet {
  /// Makes a set that holds no rules.
  #[allow(dead_code)]
  pub(crate) fn new() -> Self {
    PreRuleSet { rules: vec![] }
  }

  pub(crate) fn create(rules: Vec<PreRules>) -> PreRules {
    let mut flat_rules = rules
      .into_iter()
      .flat_map(|rule| match rule {
        PreRules::PreRuleSet(rule_set) => rule_set.rules,
        _ => vec![rule],
      })
      .collect::<Vec<PreRules>>();

    match flat_rules.len() {
      0 => PreRules::NullPreRule(NullPreRule::new()),
      1 => flat_rules.swap_remove(0),
      _ => PreRules::PreRuleSet(PreRuleSet { rules: flat_rules }),
    }
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl PreRule for PreRuleSet {
  /// Equal when both hold equal rules in the same order.
  fn equals(&self, other: &PreRules) -> bool {
    match other {
      PreRules::PreRuleSet(other) => {
        self.rules.len() == other.rules.len()
          && std::iter::zip(&self.rules, &other.rules).all(|(left, right)| left.equals(right))
      },
      _ => false,
    }
  }
  fn compiled(&mut self, state: &mut StateManager) -> CompiledResult {
    let style_tuple = self
      .rules
      .iter_mut()
      .flat_map(|rule| {
        let compiled_rule = match rule {
          PreRules::PreRuleSet(rule_set) => rule_set.compiled(state),
          PreRules::StylesPreRule(styles_pre_rule) => styles_pre_rule.compiled(state),
          PreRules::NullPreRule(null_pre_rule) => null_pre_rule.compiled(state),
        };

        match compiled_rule {
          CompiledResult::ComputedStyles(styles) => styles,
          _ => vec![],
        }
      })
      .collect::<Vec<ComputedStyle>>();

    CompiledResult::ComputedStyles(style_tuple)
  }
  fn get_value(&self) -> Option<PreRuleValue> {
    let rule = match self.rules.first() {
      Some(r) => r,
      None => stylex_panic!("{}", RULE_SET_EMPTY),
    };

    match &rule {
      PreRules::PreRuleSet(rule_set) => rule_set.get_value(),
      PreRules::StylesPreRule(styles_pre_rule) => styles_pre_rule.get_value(),
      PreRules::NullPreRule(null_pre_rule) => null_pre_rule.get_value(),
    }
  }
}
