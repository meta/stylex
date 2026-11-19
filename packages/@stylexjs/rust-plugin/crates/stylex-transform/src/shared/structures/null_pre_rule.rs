use super::pre_rule::{CompiledResult, PreRule, PreRules};
use stylex_state::state_manager::StateManager;
use stylex_structures::pre_rule_value::PreRuleValue;

#[derive(Debug, Clone, PartialEq, Copy, Default)]
pub(crate) struct NullPreRule {}

impl NullPreRule {
  #[must_use]
  pub(crate) fn new() -> Self {
    Self::default()
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl PreRule for NullPreRule {
  fn get_value(&self) -> Option<PreRuleValue> {
    None
  }
  fn compiled(&mut self, _: &mut StateManager) -> CompiledResult {
    CompiledResult::Null
  }
  /// Every null rule stands for the same absence, so any two of them are equal.
  fn equals(&self, other: &PreRules) -> bool {
    matches!(other, PreRules::NullPreRule(_))
  }
}
