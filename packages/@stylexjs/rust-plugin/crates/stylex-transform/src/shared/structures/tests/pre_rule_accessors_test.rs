//! What the field readers of a pre-rule answer.
//!
//! The readers are `CompiledResult::as_computed_styles`, the three field
//! readers of `StylesPreRule`, and `PreRuleSet::new`. No production code calls
//! them, so these tests are the only guard on what they return. Ticket 31
//! keeps the five functions and records them.
//!
//! Each reader gets a usual key path, an unusual one, and a limit case. The
//! limit cases are here because a key path comes from author source, and
//! author source can hold a very long path or an empty string.
//!
//! The key-path cases read a rule that `StylesPreRule::new` built, so they
//! measure the split that the constructor makes as well as the reader that
//! reports it. That is the only route to a built rule, and a reader that
//! disagreed with the constructor would fail here.

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

  /// The value never matters to a reader below, so the builder fixes it.
  fn rule(property: &str, key_path: Option<&[&str]>) -> StylesPreRule {
    styles(property, "red", key_path)
  }

  /// The same rule, in the enum that `PreRuleSet::create` takes.
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

    /// An empty list and a null result are different answers. A caller that
    /// reads `None` for both cannot tell "no styles" from "not a style
    /// result", so the empty list must stay `Some`.
    #[test]
    fn tells_an_empty_list_from_a_null_result() {
      let empty = CompiledResult::ComputedStyles(vec![]);

      assert_eq!(empty.as_computed_styles(), Some(&vec![]));
      assert_ne!(
        empty.as_computed_styles(),
        CompiledResult::Null.as_computed_styles()
      );
    }

    /// A namespace can compile to many classes. The reader must give back
    /// every one, in the order it got them.
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

    /// The reader never refuses. It reports what the rule holds, even when the
    /// author wrote an empty property name, so a caller sees the real value
    /// instead of a silent `None`.
    #[test]
    fn gives_an_empty_property_name_unchanged() {
      assert_eq!(rule("", None).property(), Some(""));
    }

    /// A custom property is a normal property name here. The reader must not
    /// treat the leading dashes as a marker.
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

    /// The two readers split one key path. A key belongs to one of them or to
    /// neither, and never to both.
    #[test]
    fn splits_a_key_path_between_the_two_readers() {
      let rule = rule("color", Some(&[":hover", "@media (min-width: 1px)"]));

      assert_eq!(rule.pseudos(), Some(vec![":hover".to_string()]));
      assert_eq!(
        rule.at_rules(),
        Some(vec!["@media (min-width: 1px)".to_string()])
      );
    }

    /// An attribute selector is a pseudo for this reader, because selector
    /// assembly needs it beside the colon keys.
    #[test]
    fn counts_an_attribute_selector_as_a_pseudo() {
      let rule = rule("color", Some(&["[data-active]"]));

      assert_eq!(rule.pseudos(), Some(vec!["[data-active]".to_string()]));
      assert_eq!(rule.at_rules(), Some(vec![]));
    }

    /// A pseudo element carries two colons and still belongs to the pseudo
    /// reader.
    #[test]
    fn counts_a_pseudo_element_as_a_pseudo() {
      let rule = rule("color", Some(&["::before"]));

      assert_eq!(rule.pseudos(), Some(vec!["::before".to_string()]));
    }

    /// A key that names neither kind reaches neither reader. A `var(--...)`
    /// key belongs to the const rules, and a bare word belongs to nothing.
    #[test]
    fn drops_a_key_that_names_neither_kind() {
      let rule = rule("color", Some(&["var(--theme)", "default", "1"]));

      assert_eq!(rule.pseudos(), Some(vec![]));
      assert_eq!(rule.at_rules(), Some(vec![]));
    }

    /// A key path may hold the same key more than once. The readers keep every
    /// copy, because removing one would change the selector that is built.
    #[test]
    fn keeps_a_repeated_key() {
      let rule = rule("color", Some(&[":hover", ":hover"]));

      assert_eq!(
        rule.pseudos(),
        Some(vec![":hover".to_string(), ":hover".to_string()])
      );
    }

    /// Author source can hold any text. The readers must pass a very long key
    /// and a non-ASCII key through without a change.
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

    /// A very long key path must not crash the readers, and each key must
    /// reach the reader that owns it.
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

    /// The readers copy the fields. A caller that changes what it got back
    /// must not change the rule.
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

    /// Only `new` can make an empty set. `create` collapses an empty list to a
    /// null rule instead, so no other route reaches this value. That is why
    /// the function has no production caller.
    #[test]
    fn is_a_value_that_create_cannot_make() {
      assert_eq!(
        PreRuleSet::create(vec![]),
        PreRules::NullPreRule(NullPreRule::new())
      );
    }

    /// A set of one rule collapses to that rule, so the shortest list that
    /// `create` answers with a set holds two. An empty set is below that floor.
    #[test]
    fn is_shorter_than_the_shortest_set_create_answers_with() {
      let one = PreRuleSet::create(vec![rule_of("color")]);
      let two = PreRuleSet::create(vec![rule_of("color"), rule_of("marginTop")]);

      assert_eq!(one, rule_of("color"));
      assert!(matches!(two, PreRules::PreRuleSet(_)));
    }

    /// `new` is the only route to the value, and it holds no rule.
    #[test]
    fn holds_no_rule_and_equals_another_empty_set() {
      let empty = PreRuleSet::new();

      assert_eq!(empty, PreRuleSet::new());
      assert_ne!(PreRules::PreRuleSet(empty), rule_of("color"));
    }

    /// A set is flattened into the list that holds it before the length is
    /// read, so an empty set contributes nothing and a list holding only that
    /// set collapses to a null rule.
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
