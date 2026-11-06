//! Field readers of a pre-rule: `CompiledResult::as_computed_styles`, the `StylesPreRule`
//! readers, and `PreRuleSet::new`. No production code calls them; these tests are their guard.

#[cfg(test)]
mod pre_rule_accessors {
  use indexmap::IndexMap;

  use crate::shared::structures::{
    null_pre_rule::NullPreRule,
    pre_rule::{CompiledResult, ComputedStyle, PreRules, StylesPreRule},
    pre_rule_set::PreRuleSet,
    tests::prelude::styles,
  };
  use stylex_types::structures::{injectable_style::InjectableStyle, style_key::ClassName};

  fn rule(property: &str, key_path: Option<&[&str]>) -> StylesPreRule {
    styles(property, "red", key_path)
  }

  fn rule_of(property: &str) -> PreRules {
    PreRules::StylesPreRule(rule(property, None))
  }

  fn computed_style(class_name: &str) -> ComputedStyle {
    ComputedStyle(
      ClassName(class_name.to_string()),
      InjectableStyle {
        ltr: format!(".{class_name}{{color:red}}"),
        rtl: None,
        priority: Some(3000.0),
      },
      IndexMap::new(),
    )
  }

  mod a_compiled_result {
    use super::*;

    #[test]
    fn gives_the_styles_it_holds() {
      let result = CompiledResult::ComputedStyles(vec![computed_style("x1")]);

      let styles = result.as_computed_styles();

      assert_eq!(styles.map(Vec::len), Some(1));
      assert_eq!(styles.and_then(|s| s.first()), Some(&computed_style("x1")));
    }

    #[test]
    fn gives_nothing_for_a_null_result() {
      assert_eq!(CompiledResult::Null.as_computed_styles(), None);
    }

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

    #[test]
    fn keeps_the_order_of_a_long_list() {
      let styles: Vec<ComputedStyle> = (0..5_000)
        .map(|index| computed_style(&format!("x{index}")))
        .collect();

      let result = CompiledResult::ComputedStyles(styles.clone());

      assert_eq!(result.as_computed_styles(), Some(&styles));
    }
  }

  mod a_styles_rule {
    use super::*;

    #[test]
    fn gives_its_property_name() {
      assert_eq!(rule("color", None).property(), Some("color"));
    }

    #[test]
    fn gives_an_empty_property_name_unchanged() {
      assert_eq!(rule("", None).property(), Some(""));
    }

    /// Leading dashes are not a marker.
    #[test]
    fn gives_a_custom_property_name_unchanged() {
      assert_eq!(rule("--my-color", None).property(), Some("--my-color"));
    }

    #[test]
    fn gives_no_pseudos_and_no_at_rules_without_a_key_path() {
      let rule = rule("color", None);

      assert_eq!(rule.pseudos(), Some(vec![]));
      assert_eq!(rule.at_rules(), Some(vec![]));
    }

    #[test]
    fn gives_no_pseudos_and_no_at_rules_for_an_empty_key_path() {
      let rule = rule("color", Some(&[]));

      assert_eq!(rule.pseudos(), Some(vec![]));
      assert_eq!(rule.at_rules(), Some(vec![]));
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

    /// Selector assembly needs attribute selectors beside the colon keys.
    #[test]
    fn counts_an_attribute_selector_as_a_pseudo() {
      let rule = rule("color", Some(&["[data-active]"]));

      assert_eq!(rule.pseudos(), Some(vec!["[data-active]".to_string()]));
      assert_eq!(rule.at_rules(), Some(vec![]));
    }

    #[test]
    fn counts_a_pseudo_element_as_a_pseudo() {
      let rule = rule("color", Some(&["::before"]));

      assert_eq!(rule.pseudos(), Some(vec!["::before".to_string()]));
    }

    /// A `var(--...)` key belongs to the const rules; a bare word belongs to nothing.
    #[test]
    fn drops_a_key_that_names_neither_kind() {
      let rule = rule("color", Some(&["var(--theme)", "default", "1"]));

      assert_eq!(rule.pseudos(), Some(vec![]));
      assert_eq!(rule.at_rules(), Some(vec![]));
    }

    /// Removing a copy would change the built selector.
    #[test]
    fn keeps_a_repeated_key() {
      let rule = rule("color", Some(&[":hover", ":hover"]));

      assert_eq!(
        rule.pseudos(),
        Some(vec![":hover".to_string(), ":hover".to_string()])
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

    #[test]
    fn reads_a_very_long_key_path() {
      let keys: Vec<String> = (0..10_000)
        .map(|index| {
          if index % 2 == 0 {
            format!(":nth-child({index})")
          } else {
            format!("@media (min-width: {index}px)")
          }
        })
        .collect();
      let borrowed: Vec<&str> = keys.iter().map(String::as_str).collect();

      let rule = rule("color", Some(&borrowed));

      assert_eq!(rule.pseudos().map(|keys| keys.len()), Some(5_000));
      assert_eq!(rule.at_rules().map(|keys| keys.len()), Some(5_000));
    }

    #[test]
    fn gives_a_copy_and_not_the_field() {
      let rule = rule("color", Some(&[":hover"]));

      let mut taken = rule.pseudos().unwrap_or_default();
      taken.push(":focus".to_string());

      assert_eq!(rule.pseudos(), Some(vec![":hover".to_string()]));
    }
  }

  mod an_empty_rule_set {
    use super::*;

    /// `create` collapses an empty list to a null rule, so only `new` reaches this value.
    #[test]
    fn is_a_value_that_create_cannot_make() {
      assert_eq!(
        PreRuleSet::create(vec![]),
        PreRules::NullPreRule(NullPreRule::new())
      );
    }

    /// A one-rule list collapses to that rule, so `create` sets hold at least two.
    #[test]
    fn is_shorter_than_the_shortest_set_create_answers_with() {
      let one = PreRuleSet::create(vec![rule_of("color")]);
      let two = PreRuleSet::create(vec![rule_of("color"), rule_of("marginTop")]);

      assert_eq!(one, rule_of("color"));
      assert!(matches!(two, PreRules::PreRuleSet(_)));
    }

    #[test]
    fn holds_no_rule_and_equals_another_empty_set() {
      let empty = PreRuleSet::new();

      assert_eq!(empty, PreRuleSet::new());
      assert_ne!(PreRules::PreRuleSet(empty), rule_of("color"));
    }

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
