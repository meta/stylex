#[cfg(test)]
mod flatten_style_object_with_legacy_shorthand_expansion {

  use stylex_structures::pre_rule_value::PreRuleValue;

  use crate::{
    shared::structures::pre_rule::PreRules, shared::structures::pre_rule::StylesPreRule,
    shared::structures::pre_rule_set::PreRuleSet,
  };

  use stylex_enums::style_resolution::StyleResolution;

  use stylex_state::state_manager::StateManager;

  pub(super) fn get_state() -> StateManager {
    let mut state_manager = StateManager::default();

    state_manager.options = state_manager
      .options
      .with_class_name_prefix("x")
      .with_style_resolution(StyleResolution::LegacyExpandShorthands)
      .with_runtime_injection(None)
      .with_dev(false)
      .with_test(false)
      .with_debug(false);

    state_manager
  }

  pub(super) fn pre_rule_factory(key: &str, value: &str, path_key: &[&str]) -> PreRules {
    PreRules::StylesPreRule(StylesPreRule::new(
      key,
      PreRuleValue::string(value),
      Some(path_key.iter().map(|s| s.to_string()).collect()),
    ))
  }

  pub(super) fn pre_rule_set_factory(values: &[PreRules]) -> PreRules {
    PreRuleSet::create(values.to_vec())
  }
}

#[cfg(test)]
mod multiple_levels_of_nesting {
  use indexmap::IndexMap;

  use stylex_ast::ast::factories::{
    create_key_value_prop_ident,
    create_nested_object_prop,
    create_object_expression,
    create_string_key_value_prop,
  };
  use stylex_state::functions::FunctionMap;
  use stylex_evaluator::state::EvaluationState;
  use crate::shared::structures::tests::flatten_raw_style_objects_test::flatten_style_object_with_legacy_shorthand_expansion::{
    get_state,
    pre_rule_factory,
    pre_rule_set_factory,
  };
  use crate::shared::utils::core::flatten_raw_style_object::flatten_raw_style_object;

  #[test]
  fn extra_deep_pseudo_within_a_media_query_modern_syntax() {
    let result = flatten_raw_style_object(
      &[create_key_value_prop_ident(
        "color",
        create_object_expression(vec![
          create_string_key_value_prop("default", "blue"),
          create_nested_object_prop(
            "@media (min-width: 300px)",
            vec![create_nested_object_prop(
              ":hover",
              vec![
                create_string_key_value_prop("default", "red"),
                create_string_key_value_prop(":active", "maroon"),
              ],
            )],
          ),
        ]),
      )],
      &mut EvaluationState::new(),
      &mut get_state(),
      &FunctionMap::default(),
    );

    assert_eq!(result.len(), 1);

    let mut expected_result = IndexMap::new();

    expected_result.insert(
      "color".to_string(),
      pre_rule_set_factory(&[
        pre_rule_factory("color", "blue", &["color", "default"]),
        pre_rule_factory(
          "color",
          "red",
          &["color", "@media (min-width: 300px)", ":hover", "default"],
        ),
        pre_rule_factory(
          "color",
          "maroon",
          &["color", "@media (min-width: 300px)", ":hover", ":active"],
        ),
      ]),
    );

    assert_eq!(result, expected_result)
  }
}
