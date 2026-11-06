#[cfg(test)]
mod fold_to_rule_name {
  use std::rc::Rc;

  use swc_core::ecma::ast::Expr;

  use crate::shared::transformers::named_rule::fold_to_rule_name;
  use stylex_ast::ast::convertors::create_string_expr;
  use stylex_state::{evaluate_result_value::EvaluateResultValue, state_manager::StateManager};
  use stylex_types::{
    enums::data_structures::injectable_style::InjectableStyleKind,
    structures::injectable_style::InjectableStyle,
  };

  /// A rule with the given name and CSS text, which is all the fold reads.
  fn rule(name: &str, css: &str) -> (String, InjectableStyleKind) {
    (
      name.to_string(),
      InjectableStyleKind::Regular(InjectableStyle {
        ltr: css.to_string(),
        rtl: None,
        priority: Some(1.0),
      }),
    )
  }

  /// Stands for a transformer that names its rule after the string it was
  /// given, so a test can choose the name the fold must use.
  fn name_from_expr(
    styles: &EvaluateResultValue,
    _: &mut StateManager,
  ) -> (String, InjectableStyleKind) {
    let name = match styles.as_expr().and_then(|expr| match expr {
      Expr::Lit(lit) => stylex_ast::ast::convertors::convert_lit_to_string(lit),
      _ => None,
    }) {
      Some(text) => text,
      None => "unnamed".to_string(),
    };

    rule(&name, "@keyframes x{}")
  }

  fn injected_names(state: &StateManager) -> Vec<String> {
    state
      .other_injected_css_rules
      .keys()
      .map(|key| key.to_string())
      .collect()
  }

  #[test]
  fn folds_the_call_to_the_name_the_transformer_chose() {
    let mut state = StateManager::default();

    let folded = fold_to_rule_name(create_string_expr("xanim"), &mut state, name_from_expr);

    assert_eq!(folded, create_string_expr("xanim"));
    assert_eq!(injected_names(&state), vec!["xanim".to_string()]);
  }

  #[test]
  fn keeps_the_rule_under_the_same_name_it_folds_to() {
    let mut state = StateManager::default();

    fold_to_rule_name(create_string_expr("xanim"), &mut state, name_from_expr);

    let key: stylex_types::structures::style_key::RuleKey = "xanim".into();
    let kept = state.other_injected_css_rules.get(&key).cloned();

    assert_eq!(kept, Some(Rc::new(rule("xanim", "@keyframes x{}").1)));
  }

  #[test]
  fn two_different_names_both_stay_in_source_order() {
    let mut state = StateManager::default();

    fold_to_rule_name(create_string_expr("first"), &mut state, name_from_expr);
    fold_to_rule_name(create_string_expr("second"), &mut state, name_from_expr);

    assert_eq!(
      injected_names(&state),
      vec!["first".to_string(), "second".to_string()]
    );
  }

  /// The same content twice is one rule, because the name is the key.
  #[test]
  fn the_same_name_twice_stays_one_entry() {
    let mut state = StateManager::default();

    fold_to_rule_name(create_string_expr("same"), &mut state, name_from_expr);
    fold_to_rule_name(create_string_expr("same"), &mut state, name_from_expr);

    assert_eq!(injected_names(&state), vec!["same".to_string()]);
  }

  /// A rule the fold puts in does not remove what the state already holds.
  #[test]
  fn keeps_a_rule_that_was_there_before() {
    let mut state = StateManager::default();

    state
      .other_injected_css_rules
      .insert("earlier".into(), Rc::new(rule("earlier", ".a{}").1));

    fold_to_rule_name(create_string_expr("later"), &mut state, name_from_expr);

    assert_eq!(
      injected_names(&state),
      vec!["earlier".to_string(), "later".to_string()]
    );
  }

  /// An expression the transformer cannot name is still folded, to whatever
  /// name the transformer fell back to.
  #[test]
  fn folds_an_expression_the_transformer_cannot_name() {
    let mut state = StateManager::default();

    let folded = fold_to_rule_name(
      stylex_ast::ast::factories::create_object_expression(vec![]),
      &mut state,
      name_from_expr,
    );

    assert_eq!(folded, create_string_expr("unnamed"));
    assert_eq!(injected_names(&state), vec!["unnamed".to_string()]);
  }

  /// An empty name is a legal key, so it is kept like any other.
  #[test]
  fn accepts_an_empty_name() {
    let mut state = StateManager::default();

    let folded = fold_to_rule_name(create_string_expr(""), &mut state, name_from_expr);

    assert_eq!(folded, create_string_expr(""));
    assert_eq!(injected_names(&state), vec![String::new()]);
  }

  /// A name is a key, not CSS, so nothing in it is escaped or refused.
  #[test]
  fn accepts_a_name_with_unusual_characters() {
    let mut state = StateManager::default();

    let odd = "\u{1F600} ünïcødé\t\"{};";

    let folded = fold_to_rule_name(create_string_expr(odd), &mut state, name_from_expr);

    assert_eq!(folded, create_string_expr(odd));
    assert_eq!(injected_names(&state), vec![odd.to_string()]);
  }

  /// A name far longer than any hash the transformers make.
  #[test]
  fn accepts_a_very_long_name() {
    let mut state = StateManager::default();

    let long = "x".repeat(100_000);

    let folded = fold_to_rule_name(create_string_expr(&long), &mut state, name_from_expr);

    assert_eq!(folded, create_string_expr(&long));
    assert_eq!(injected_names(&state), vec![long]);
  }

  /// Many rules in one file, to show the fold adds one entry per call and
  /// holds their order across a large map.
  #[test]
  fn keeps_ten_thousand_rules_in_order() {
    let mut state = StateManager::default();

    for index in 0..10_000 {
      fold_to_rule_name(
        create_string_expr(&format!("rule{index}")),
        &mut state,
        name_from_expr,
      );
    }

    let names = injected_names(&state);

    assert_eq!(names.len(), 10_000);
    assert_eq!(names.first(), Some(&"rule0".to_string()));
    assert_eq!(names.last(), Some(&"rule9999".to_string()));
  }
}
