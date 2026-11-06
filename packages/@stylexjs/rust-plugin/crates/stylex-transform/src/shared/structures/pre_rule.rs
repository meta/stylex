use std::fmt::Debug;

use indexmap::IndexMap;

use crate::shared::utils::core::convert_style_to_class_name::convert_style_to_class_name;
use stylex_css::utils::{
  pre_rule::{sort_at_rules, sort_pseudos},
  pseudo::is_pseudo_selector,
};
use stylex_state::{state_manager::StateManager, types::ClassNameToOriginalPaths};
use stylex_types::structures::style_key::ClassName;

use super::{null_pre_rule::NullPreRule, pre_rule_set::PreRuleSet};
use stylex_structures::pre_rule_value::PreRuleValue;
use stylex_types::structures::injectable_style::InjectableStyle;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ComputedStyle(
  pub(crate) ClassName,
  pub(crate) InjectableStyle,
  pub(crate) ClassNameToOriginalPaths,
);

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CompiledResult {
  Null,
  ComputedStyles(Vec<ComputedStyle>),
}

impl CompiledResult {
  /// Gets the styles in this result. Gives `None` for a null result.
  #[allow(dead_code)]
  pub(crate) fn as_computed_styles(&self) -> Option<&Vec<ComputedStyle>> {
    match self {
      CompiledResult::ComputedStyles(computed_styles) => Some(computed_styles),
      _ => None,
    }
  }
}

pub(crate) trait PreRule: Debug {
  #[allow(dead_code)]
  fn get_value(&self) -> Option<PreRuleValue>;
  fn compiled(&mut self, state: &mut StateManager) -> CompiledResult;
  /// Whether `other` is the same rule (the reference implementation's `instanceof` check).
  #[allow(dead_code)]
  fn equals(&self, other: &PreRules) -> bool;
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PreRules {
  PreRuleSet(PreRuleSet),
  StylesPreRule(StylesPreRule),
  NullPreRule(NullPreRule),
}

impl PreRules {
  /// [`PreRule::equals`], asked of whichever rule this variant holds.
  #[allow(dead_code)]
  pub(crate) fn equals(&self, other: &PreRules) -> bool {
    match self {
      PreRules::PreRuleSet(rule_set) => rule_set.equals(other),
      PreRules::StylesPreRule(styles_pre_rule) => styles_pre_rule.equals(other),
      PreRules::NullPreRule(null_pre_rule) => null_pre_rule.equals(other),
    }
  }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StylesPreRule {
  property: String,
  value: PreRuleValue,
  pseudos: Vec<String>,
  at_rules: Vec<String>,
  const_rules: Vec<String>,
  key_path: Vec<String>,
}

impl StylesPreRule {
  /// Collects the key path segments `keep` selects, in authored order.
  fn select_key_path(key_path: &Option<Vec<String>>, keep: impl Fn(&str) -> bool) -> Vec<String> {
    key_path
      .iter()
      .flatten()
      .filter(|key| keep(key))
      .cloned()
      .collect()
  }

  fn get_pseudos(key_path: &Option<Vec<String>>) -> Vec<String> {
    // Single colon on purpose: selectors need pseudo classes as well as pseudo elements.
    let unsorted_pseudos = Self::select_key_path(key_path, |key| {
      is_pseudo_selector(key) || key.starts_with('[')
    });

    sort_pseudos(&unsorted_pseudos)
  }

  fn get_at_rules(key_path: &Option<Vec<String>>) -> Vec<String> {
    let unsorted_at_rules = Self::select_key_path(key_path, |key| key.starts_with('@'));

    sort_at_rules(&unsorted_at_rules)
  }

  /// Placeholder wrappers keep the authored path; conditions are sorted when hashing.
  fn get_const_rules(key_path: &Option<Vec<String>>) -> Vec<String> {
    Self::select_key_path(key_path, |key| key.starts_with("var(--"))
  }
  pub(crate) fn new(property: &str, value: PreRuleValue, key_path: Option<Vec<String>>) -> Self {
    let property_str = property.to_string();

    StylesPreRule {
      property: property_str,
      value,
      pseudos: StylesPreRule::get_pseudos(&key_path),
      at_rules: StylesPreRule::get_at_rules(&key_path),
      const_rules: StylesPreRule::get_const_rules(&key_path),
      key_path: key_path.unwrap_or_default(),
    }
  }
  // Field getters below are unused in production.

  /// Gets the CSS property name of this rule.
  #[allow(dead_code)]
  pub(crate) fn property(&self) -> Option<&str> {
    Some(&self.property)
  }

  /// Gets the pseudo selectors that the key path gave to this rule.
  #[allow(dead_code)]
  pub(crate) fn pseudos(&self) -> Option<Vec<String>> {
    Some(self.pseudos.to_owned())
  }

  /// Gets the at-rules that the key path gave to this rule.
  #[allow(dead_code)]
  pub(crate) fn at_rules(&self) -> Option<Vec<String>> {
    Some(self.at_rules.to_owned())
  }
}

impl PreRule for StylesPreRule {
  #[cfg_attr(coverage_nightly, coverage(off))]
  fn get_value(&self) -> Option<PreRuleValue> {
    Some(self.value.to_owned())
  }

  fn compiled(&mut self, state: &mut StateManager) -> CompiledResult {
    let Some((_, class_name, rule)) = convert_style_to_class_name(
      (self.property.as_str(), &self.value),
      &mut self.pseudos,
      &mut self.at_rules,
      &mut self.const_rules,
      state,
    ) else {
      return CompiledResult::Null;
    };

    let mut classes_to_original_paths = IndexMap::new();

    classes_to_original_paths.insert(class_name.clone(), self.key_path.clone());

    CompiledResult::ComputedStyles(vec![ComputedStyle(
      class_name,
      rule,
      classes_to_original_paths,
    )])
  }

  /// Compares property, value and the two sorted key-path slices.
  fn equals(&self, other: &PreRules) -> bool {
    match other {
      PreRules::StylesPreRule(other) => {
        self.property == other.property
          && self.value == other.value
          && self.pseudos == other.pseudos
          && self.at_rules == other.at_rules
      },
      _ => false,
    }
  }
}
