//! Field readers of a pre-rule: `CompiledResult::as_computed_styles`, the `StylesPreRule`
//! readers, and `PreRuleSet::new`. No production code calls them; these tests are their guard.

#[cfg(test)]
mod pre_rule_accessors {

  use crate::shared::structures::{
    null_pre_rule::NullPreRule,
    pre_rule::{CompiledResult, PreRules, StylesPreRule},
    pre_rule_set::PreRuleSet,
    tests::prelude::styles,
  };

  fn rule(property: &str, key_path: Option<&[&str]>) -> StylesPreRule {
    styles(property, "red", key_path)
  }

  fn rule_of(property: &str) -> PreRules {
    PreRules::StylesPreRule(rule(property, None))
  }

  mod a_compiled_result {
    use super::*;

    /// "No styles" (`Some(empty)`) must stay distinct from "not a style result" (`None`).
    #[test]
    fn tells_an_empty_list_from_a_null_result() {
      let empty = CompiledResult::ComputedStyles(vec![]);

      assert_eq!(empty.as_computed_styles(), Some(&vec![]));
      assert_ne!(
        empty.as_computed_styles(),
        CompiledResult::Null.as_computed_styles()
      );
    }
  }

  mod a_styles_rule {
    use super::*;

    /// Leading dashes are not a marker.
    #[test]
    fn gives_a_custom_property_name_unchanged() {
      assert_eq!(rule("--my-color", None).property(), Some("--my-color"));
    }

    /// A key belongs to one reader or neither, never both.
    #[test]
    fn splits_a_key_path_between_the_two_readers() {
      let rule = rule("color", Some(&[":hover", "@media (min-width: 1px)"]));

      assert_eq!(rule.pseudos(), Some(vec![":hover".to_string()]));
      assert_eq!(
        rule.at_rules(),
        Some(vec!["@media (min-width: 1px)".to_string()])
      );
    }

    #[test]
    fn gives_a_long_and_a_non_ascii_key_unchanged() {
      let long_key = format!(":{}", "a".repeat(10_000));
      let unicode_key = ":hover-日本語-🎨";
      let rule = rule("color", Some(&[long_key.as_str(), unicode_key]));

      let pseudos = rule.pseudos().unwrap_or_default();

      assert_eq!(pseudos.len(), 2);
      assert!(pseudos.contains(&long_key));
      assert!(pseudos.contains(&unicode_key.to_string()));
    }
  }

  mod an_empty_rule_set {
    use super::*;

    /// Sets are flattened before the length is read.
    #[test]
    fn contributes_no_rule_to_the_list_that_holds_it() {
      let flattened = PreRuleSet::create(vec![
        PreRules::PreRuleSet(PreRuleSet::new()),
        rule_of("color"),
      ]);

      assert_eq!(flattened, rule_of("color"));
      assert_eq!(
        PreRuleSet::create(vec![PreRules::PreRuleSet(PreRuleSet::new())]),
        PreRules::NullPreRule(NullPreRule::new())
      );
    }
  }
}
