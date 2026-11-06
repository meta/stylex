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
  // Kept by ticket 31. No production code calls it, because the rest of the
  // crate reads the enum with a match. The compiler warns without the
  // attribute.
  #[allow(dead_code)]
  pub(crate) fn as_computed_styles(&self) -> Option<&Vec<ComputedStyle>> {
    match self {
      CompiledResult::ComputedStyles(computed_styles) => Some(computed_styles),
      _ => None,
    }
  }
}

pub(crate) trait PreRule: Debug {
  // Load-bearing, measured: the only calls to `get_value` come from
  // `PreRuleSet::get_value`, which is one of its own three implementations. The
  // compiler reads that cycle as dead and warns without this line.
  #[allow(dead_code)]
  fn get_value(&self) -> Option<PreRuleValue>;
  fn compiled(&mut self, state: &mut StateManager) -> CompiledResult;
  /// Whether `other` is the same rule.
  ///
  /// `other` is the [`PreRules`] enum and not a trait object, so an
  /// implementation can read the fields of its own kind and answer `false` for
  /// the other two -- the kind test the reference implementation spells as
  /// `instanceof`.
  // Load-bearing, measured: `equals` is reached only through `PreRules::equals`,
  // which the same cycle makes unreachable to the lint.
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
  // Load-bearing, measured: the one caller is `PreRuleSet::equals`, which is an
  // implementation of the trait method this dispatches to. Neither end of the
  // cycle has an outside caller, so the lint fires on both without the line.
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
  /// Collects the key path segments a rule kind owns, in authored order.
  ///
  /// Each of the three kinds walks the same path and keeps a disjoint slice of
  /// it, so the filter is passed in rather than the path being cloned once per
  /// kind.
  fn select_key_path(key_path: &Option<Vec<String>>, keep: impl Fn(&str) -> bool) -> Vec<String> {
    key_path
      .iter()
      .flatten()
      .filter(|key| keep(key))
      .cloned()
      .collect()
  }

  fn get_pseudos(key_path: &Option<Vec<String>>) -> Vec<String> {
    // The single colon is deliberate: selector assembly needs both kinds, so
    // narrowing this to `::` would drop every pseudo class on the floor.
    let unsorted_pseudos = Self::select_key_path(key_path, |key| {
      is_pseudo_selector(key) || key.starts_with('[')
    });

    sort_pseudos(&unsorted_pseudos)
  }

  fn get_at_rules(key_path: &Option<Vec<String>>) -> Vec<String> {
    let unsorted_at_rules = Self::select_key_path(key_path, |key| key.starts_with('@'));

    sort_at_rules(&unsorted_at_rules)
  }

  /// The `var(--…)` keys of a key path.
  ///
  /// Sorted here, which is a divergence and predates this file's current shape:
  /// upstream's `get constRules()` returns the filtered path **unsorted**, and
  /// only `convertStyleToClassName` sorts, for the hash. `generateCSSRule`
  /// receives the unsorted list and nests the declaration in that order, so two
  /// `var(--…)` keys written out of alphabetical order nest one way here and the
  /// other way in Babel. The class name is unaffected — both sides re-sort the
  /// combined list before hashing — so the emitted rule differs in nesting order
  /// alone. Left as it is rather than changed alongside unrelated work; the sort
  /// is also redundant with the one the caller does.
  fn get_const_rules(key_path: &Option<Vec<String>>) -> Vec<String> {
    let unsorted_const_rules = Self::select_key_path(key_path, |key| key.starts_with("var(--"));

    sort_at_rules(&unsorted_const_rules)
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
  // The three functions below read the fields of a rule that is already built.
  // Kept by ticket 31. No production code calls them, because the crate builds
  // a rule and then reads it through the `PreRule` trait. The compiler warns
  // without the attributes.
  //
  // Each name shows its field. The two associated functions above already use
  // the names `get_pseudos` and `get_at_rules`. Those functions take a key
  // path and select from it, which is a different task.

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
  // Reached only through the trait object, which the transform builds and
  // nothing else asks for a value from.
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
      // The value carries no CSS text, so there is no declaration to name.
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

  /// The property, the value and the two sorted key-path slices. The key path
  /// itself and the `var(--…)` rules are not compared, because both are read
  /// out of the key path the other four already stand for.
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
