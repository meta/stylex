//! `PreRule::equals` for every pair of the three rule kinds, matching the reference
//! implementation's `equals` methods.

#[cfg(test)]
mod pre_rule_equality {

  use crate::shared::structures::{
    pre_rule::{PreRule, PreRules, StylesPreRule},
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
    fn differs_from_a_null_rule_and_from_a_set() {
      let left = styles("color", "red", &[]);

      assert!(!left.equals(&null()));
      assert!(!left.equals(&set(vec![rule("color", "red", &[]), null()])));
    }
  }

  mod a_null_rule {}

  mod a_rule_set {
    use super::*;

    #[test]
    fn compares_its_members_by_their_own_equals() {
      // A member's own `equals` ignores `var(--…)` keys; a field-by-field compare would not.
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
  }
}
