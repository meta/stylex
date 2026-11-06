//! What `PreRule::equals` answers, for every pair of the three rule kinds.
//!
//! The expectations are read off the reference implementation's own `equals`
//! methods: a kind test first, then the property, the value and the two sorted
//! key-path slices for a styles rule, and length plus an element-wise compare
//! for a set.

#[cfg(test)]
mod pre_rule_equality {
  use stylex_structures::pre_rule_value::PreRuleValue;

  use crate::shared::structures::{
    null_pre_rule::NullPreRule,
    pre_rule::{PreRule, PreRules, StylesPreRule},
    pre_rule_set::PreRuleSet,
    tests::prelude::{null, set},
  };

  fn styles(property: &str, value: &str, key_path: &[&str]) -> StylesPreRule {
    super::super::prelude::styles(property, value, Some(key_path))
  }

  fn rule(property: &str, value: &str, key_path: &[&str]) -> PreRules {
    PreRules::StylesPreRule(styles(property, value, key_path))
  }

  mod a_styles_rule {
    use super::*;

    #[test]
    fn equals_another_rule_with_the_same_property_value_and_keys() {
      let left = styles("color", "red", &[":hover"]);
      let right = rule("color", "red", &[":hover"]);

      assert!(left.equals(&right));
    }

    #[test]
    fn differs_by_property() {
      let left = styles("color", "red", &[]);

      assert!(!left.equals(&rule("background", "red", &[])));
    }

    #[test]
    fn differs_by_value() {
      let left = styles("color", "red", &[]);

      assert!(!left.equals(&rule("color", "blue", &[])));
    }

    #[test]
    fn differs_by_pseudo() {
      let left = styles("color", "red", &[":hover"]);

      assert!(!left.equals(&rule("color", "red", &[":focus"])));
    }

    #[test]
    fn differs_by_at_rule() {
      let left = styles("color", "red", &["@media (width > 0)"]);

      assert!(!left.equals(&rule("color", "red", &["@media (width > 1px)"])));
    }

    #[test]
    fn ignores_a_pseudo_authored_in_another_order() {
      // Both slices are sorted by the constructor, so the authored order of two
      // pseudos cannot make two otherwise identical rules differ.
      let left = styles("color", "red", &[":hover", ":focus"]);

      assert!(left.equals(&rule("color", "red", &[":focus", ":hover"])));
    }

    #[test]
    fn ignores_the_var_keys_and_the_rest_of_the_key_path() {
      // `var(--…)` keys and the raw key path are read out of the same path the
      // four compared fields already stand for, and the reference
      // implementation leaves both out of its comparison.
      let left = styles("color", "red", &["var(--a)", "someNamespace"]);

      assert!(left.equals(&rule("color", "red", &["var(--b)", "otherNamespace"])));
    }

    #[test]
    fn differs_from_a_null_rule_and_from_a_set() {
      let left = styles("color", "red", &[]);

      assert!(!left.equals(&null()));
      assert!(!left.equals(&set(vec![rule("color", "red", &[]), null()])));
    }

    #[test]
    fn compares_a_list_value_against_a_single_one() {
      let single = styles("color", "red", &[]);
      let listed = StylesPreRule::new("color", PreRuleValue::Vec(vec![]), Some(vec![]));

      assert!(!single.equals(&PreRules::StylesPreRule(listed)));
    }

    #[test]
    fn compares_an_empty_property_and_an_empty_value() {
      let left = styles("", "", &[]);

      assert!(left.equals(&rule("", "", &[])));
      assert!(!left.equals(&rule("", " ", &[])));
    }

    #[test]
    fn compares_a_very_long_value_without_truncating_it() {
      let long = "a".repeat(100_000);
      let mut nearly = long.clone();
      nearly.pop();
      nearly.push('b');

      let left = styles("content", &long, &[]);

      assert!(left.equals(&rule("content", &long, &[])));
      assert!(!left.equals(&rule("content", &nearly, &[])));
    }
  }

  mod a_null_rule {
    use super::*;

    #[test]
    fn equals_another_null_rule() {
      let left = NullPreRule::new();

      assert!(left.equals(&null()));
    }

    #[test]
    fn differs_from_a_styles_rule_and_from_a_set() {
      let left = NullPreRule::new();

      assert!(!left.equals(&rule("color", "red", &[])));
      assert!(!left.equals(&set(vec![null(), null()])));
    }
  }

  mod a_rule_set {
    use super::*;

    /// `PreRuleSet::create` collapses zero rules to a null rule and one rule to
    /// that rule, so those two answers come from the collapsed kind and never
    /// from a set at all.
    #[test]
    fn is_never_built_from_fewer_than_two_rules() {
      assert!(matches!(
        PreRuleSet::create(vec![]),
        PreRules::NullPreRule(_)
      ));
      assert!(matches!(
        PreRuleSet::create(vec![rule("color", "red", &[])]),
        PreRules::StylesPreRule(_)
      ));
    }

    #[test]
    fn equals_a_set_holding_the_same_rules_in_the_same_order() {
      let left = set(vec![rule("color", "red", &[]), rule("width", "1", &[])]);
      let right = set(vec![rule("color", "red", &[]), rule("width", "1", &[])]);

      assert!(left.equals(&right));
    }

    #[test]
    fn differs_from_the_same_rules_in_another_order() {
      let left = set(vec![rule("color", "red", &[]), rule("width", "1", &[])]);
      let right = set(vec![rule("width", "1", &[]), rule("color", "red", &[])]);

      assert!(!left.equals(&right));
    }

    #[test]
    fn differs_from_a_set_of_another_length() {
      let left = set(vec![rule("color", "red", &[]), rule("width", "1", &[])]);
      let right = set(vec![
        rule("color", "red", &[]),
        rule("width", "1", &[]),
        rule("height", "2", &[]),
      ]);

      assert!(!left.equals(&right));
    }

    #[test]
    fn compares_its_members_by_their_own_equals() {
      // The two sets differ only in a member's `var(--…)` key, which a member's
      // own `equals` ignores. A field-by-field compare of the sets would
      // answer differently.
      let left = set(vec![rule("color", "red", &["var(--a)"]), null()]);
      let right = set(vec![rule("color", "red", &["var(--b)"]), null()]);

      assert!(left.equals(&right));
    }

    #[test]
    fn differs_from_a_styles_rule_and_from_a_null_rule() {
      let left = set(vec![rule("color", "red", &[]), null()]);

      assert!(!left.equals(&rule("color", "red", &[])));
      assert!(!left.equals(&null()));
    }

    #[test]
    fn compares_a_set_of_many_thousands_of_rules() {
      let build = |odd_one_out: Option<usize>| {
        set(
          (0..20_000)
            .map(|index| {
              let value = if odd_one_out == Some(index) { "1" } else { "0" };
              rule(&format!("--property-{index}"), value, &[])
            })
            .collect(),
        )
      };

      assert!(build(None).equals(&build(None)));
      // The one member that differs is the last, so the walk has to reach it.
      assert!(!build(None).equals(&build(Some(19_999))));
    }
  }
}
