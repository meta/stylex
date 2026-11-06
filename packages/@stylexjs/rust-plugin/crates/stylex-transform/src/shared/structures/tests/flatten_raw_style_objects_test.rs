#[cfg(test)]
mod flatten_style_object_with_legacy_shorthand_expansion {
  use indexmap::IndexMap;
  use stylex_ast::ast::convertors::create_string_expr;
  use stylex_structures::pre_rule_value::PreRuleValue;
  use swc_core::ecma::ast::Expr;

  use crate::{
    shared::structures::null_pre_rule::NullPreRule, shared::structures::pre_rule::PreRules,
    shared::structures::pre_rule::StylesPreRule, shared::structures::pre_rule_set::PreRuleSet,
    shared::utils::core::flatten_raw_style_object::flatten_raw_style_object,
  };
  use stylex_ast::ast::factories::{create_array, create_key_value_prop_ident};
  use stylex_enums::style_resolution::StyleResolution;
  use stylex_evaluator::state::EvaluationState;
  use stylex_state::{functions::FunctionMap, state_manager::StateManager};

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

  pub(super) fn null_rule_factory() -> PreRules {
    PreRules::NullPreRule(NullPreRule::default())
  }

  pub(super) fn pre_rule_vec_factory(key: &str, value: &[&str], path_key: &[&str]) -> PreRules {
    PreRules::StylesPreRule(StylesPreRule::new(
      key,
      PreRuleValue::Vec(value.iter().map(|x| (*x).into()).collect()),
      Some(path_key.iter().map(|s| s.to_string()).collect()),
    ))
  }

  #[test]
  fn should_create_pre_rule_objects_for_simple_style_values() {
    let result = flatten_raw_style_object(
      &[
        create_key_value_prop_ident("color", create_string_expr("red")),
        create_key_value_prop_ident("marginStart", create_string_expr("10")),
      ],
      &mut EvaluationState::new(),
      &mut get_state(),
      &FunctionMap::default(),
    );

    assert_eq!(result.len(), 4);

    let mut expected_result = IndexMap::new();

    expected_result.insert(
      "color".to_string(),
      pre_rule_factory("color", "red", &["color"]),
    );
    expected_result.insert(
      "marginInlineStart".to_string(),
      pre_rule_factory("marginInlineStart", "10", &["marginInlineStart"]),
    );
    expected_result.insert("marginLeft".to_string(), null_rule_factory());
    expected_result.insert("marginRight".to_string(), null_rule_factory());

    assert_eq!(result, expected_result)
  }

  #[test]
  fn should_expand_simple_gap_values() {
    let result = flatten_raw_style_object(
      &[create_key_value_prop_ident("gap", create_string_expr("10"))],
      &mut EvaluationState::new(),
      &mut get_state(),
      &FunctionMap::default(),
    );

    assert_eq!(result.len(), 2);

    let mut expected_result = IndexMap::new();

    expected_result.insert(
      "rowGap".to_string(),
      pre_rule_factory("rowGap", "10", &["rowGap"]),
    );
    expected_result.insert(
      "columnGap".to_string(),
      pre_rule_factory("columnGap", "10", &["columnGap"]),
    );

    assert_eq!(result, expected_result)
  }

  #[test]
  fn should_expand_simple_contain_intrinsic_size_values() {
    let result = flatten_raw_style_object(
      &[create_key_value_prop_ident(
        "containIntrinsicSize",
        create_string_expr("10"),
      )],
      &mut EvaluationState::new(),
      &mut get_state(),
      &FunctionMap::default(),
    );

    assert_eq!(result.len(), 2);

    let mut expected_result = IndexMap::new();

    expected_result.insert(
      "containIntrinsicWidth".to_string(),
      pre_rule_factory("containIntrinsicWidth", "10", &["containIntrinsicWidth"]),
    );
    expected_result.insert(
      "containIntrinsicHeight".to_string(),
      pre_rule_factory("containIntrinsicHeight", "10", &["containIntrinsicHeight"]),
    );

    assert_eq!(result, expected_result)
  }

  #[test]
  fn should_expand_simple_gap_with_space_separated_values() {
    let result = flatten_raw_style_object(
      &[create_key_value_prop_ident(
        "gap",
        create_string_expr("10px 20px"),
      )],
      &mut EvaluationState::new(),
      &mut get_state(),
      &FunctionMap::default(),
    );

    assert_eq!(result.len(), 2);

    let mut expected_result = IndexMap::new();

    expected_result.insert(
      "rowGap".to_string(),
      pre_rule_factory("rowGap", "10px", &["rowGap"]),
    );
    expected_result.insert(
      "columnGap".to_string(),
      pre_rule_factory("columnGap", "20px", &["columnGap"]),
    );

    assert_eq!(result, expected_result)
  }

  #[test]
  fn should_expand_simple_contain_intrinsic_size_with_space_separated_values() {
    let w = "containIntrinsicWidth";
    let h = "containIntrinsicHeight";

    let result = flatten_raw_style_object(
      &[create_key_value_prop_ident(
        "containIntrinsicSize",
        create_string_expr("10px 20px"),
      )],
      &mut EvaluationState::new(),
      &mut get_state(),
      &FunctionMap::default(),
    );

    assert_eq!(result.len(), 2);

    let mut expected_result = IndexMap::new();

    expected_result.insert(w.to_string(), pre_rule_factory(w, "10px", &[w]));
    expected_result.insert(h.to_string(), pre_rule_factory(h, "20px", &[h]));

    assert_eq!(result, expected_result)
  }

  #[test]
  fn should_expand_simple_contain_intrinsic_size_with_space_separated_values_v2() {
    let w = "containIntrinsicWidth";
    let h = "containIntrinsicHeight";

    let result = flatten_raw_style_object(
      &[create_key_value_prop_ident(
        "containIntrinsicSize",
        create_string_expr("auto 10px 20px"),
      )],
      &mut EvaluationState::new(),
      &mut get_state(),
      &FunctionMap::default(),
    );

    assert_eq!(result.len(), 2);

    let mut expected_result = IndexMap::new();

    expected_result.insert(w.to_string(), pre_rule_factory(w, "auto 10px", &[w]));
    expected_result.insert(h.to_string(), pre_rule_factory(h, "20px", &[h]));

    assert_eq!(result, expected_result)
  }

  #[test]
  fn should_expand_simple_contain_intrinsic_size_with_space_separated_values_v3() {
    let w = "containIntrinsicWidth";
    let h = "containIntrinsicHeight";

    let result = flatten_raw_style_object(
      &[create_key_value_prop_ident(
        "containIntrinsicSize",
        create_string_expr("auto 10px auto 20px"),
      )],
      &mut EvaluationState::new(),
      &mut get_state(),
      &FunctionMap::default(),
    );

    assert_eq!(result.len(), 2);

    let mut expected_result = IndexMap::new();

    expected_result.insert(w.to_string(), pre_rule_factory(w, "auto 10px", &[w]));
    expected_result.insert(h.to_string(), pre_rule_factory(h, "auto 20px", &[h]));

    assert_eq!(result, expected_result)
  }

  #[test]
  fn should_expand_simple_shorthands() {
    let result = flatten_raw_style_object(
      &[create_key_value_prop_ident(
        "margin",
        create_string_expr("10"),
      )],
      &mut EvaluationState::new(),
      &mut get_state(),
      &FunctionMap::default(),
    );

    assert_eq!(result.len(), 4);

    let mut expected_result = IndexMap::new();

    expected_result.insert(
      "marginTop".to_string(),
      pre_rule_factory("marginTop", "10", &["marginTop"]),
    );
    expected_result.insert(
      "marginInlineEnd".to_string(),
      pre_rule_factory("marginInlineEnd", "10", &["marginInlineEnd"]),
    );
    expected_result.insert(
      "marginBottom".to_string(),
      pre_rule_factory("marginBottom", "10", &["marginBottom"]),
    );
    expected_result.insert(
      "marginInlineStart".to_string(),
      pre_rule_factory("marginInlineStart", "10", &["marginInlineStart"]),
    );

    assert_eq!(result, expected_result)
  }

  #[test]
  fn should_expand_simple_shorthands_extended() {
    let result = flatten_raw_style_object(
      &[
        create_key_value_prop_ident("margin", create_string_expr("10")),
        create_key_value_prop_ident("marginBottom", create_string_expr("20")),
      ],
      &mut EvaluationState::new(),
      &mut get_state(),
      &FunctionMap::default(),
    );

    assert_eq!(result.len(), 4);

    let mut expected_result = IndexMap::new();

    expected_result.insert(
      "marginTop".to_string(),
      pre_rule_factory("marginTop", "10", &["marginTop"]),
    );
    expected_result.insert(
      "marginInlineEnd".to_string(),
      pre_rule_factory("marginInlineEnd", "10", &["marginInlineEnd"]),
    );
    expected_result.insert(
      "marginBottom".to_string(),
      pre_rule_factory("marginBottom", "10", &["marginBottom"]),
    );
    expected_result.insert(
      "marginInlineStart".to_string(),
      pre_rule_factory("marginInlineStart", "10", &["marginInlineStart"]),
    );

    expected_result.insert(
      "marginBottom".to_string(),
      pre_rule_factory("marginBottom", "20", &["marginBottom"]),
    );

    assert_eq!(result, expected_result)
  }

  #[test]
  fn should_expand_shorthands_with_space_separated_values() {
    let result = flatten_raw_style_object(
      &[
        create_key_value_prop_ident("margin", create_string_expr("10px 20px")),
        create_key_value_prop_ident("borderColor", create_string_expr("red")),
      ],
      &mut EvaluationState::new(),
      &mut get_state(),
      &FunctionMap::default(),
    );

    assert_eq!(result.len(), 8);

    let mut expected_result = IndexMap::new();

    expected_result.insert(
      "marginTop".to_string(),
      pre_rule_factory("marginTop", "10px", &["marginTop"]),
    );
    expected_result.insert(
      "marginInlineEnd".to_string(),
      pre_rule_factory("marginInlineEnd", "20px", &["marginInlineEnd"]),
    );
    expected_result.insert(
      "marginBottom".to_string(),
      pre_rule_factory("marginBottom", "10px", &["marginBottom"]),
    );
    expected_result.insert(
      "marginInlineStart".to_string(),
      pre_rule_factory("marginInlineStart", "20px", &["marginInlineStart"]),
    );

    expected_result.insert(
      "borderTopColor".to_string(),
      pre_rule_factory("borderTopColor", "red", &["borderTopColor"]),
    );
    expected_result.insert(
      "borderInlineEndColor".to_string(),
      pre_rule_factory("borderInlineEndColor", "red", &["borderInlineEndColor"]),
    );
    expected_result.insert(
      "borderBottomColor".to_string(),
      pre_rule_factory("borderBottomColor", "red", &["borderBottomColor"]),
    );
    expected_result.insert(
      "borderInlineStartColor".to_string(),
      pre_rule_factory("borderInlineStartColor", "red", &["borderInlineStartColor"]),
    );

    assert_eq!(result, expected_result)
  }

  #[test]
  fn should_expand_shorthands_with_fallbacks() {
    let result = flatten_raw_style_object(
      &[create_key_value_prop_ident(
        "margin",
        Expr::from(create_array(&[
          create_string_expr("10vh 20px"),
          create_string_expr("10dvh 20px"),
        ])),
      )],
      &mut EvaluationState::new(),
      &mut get_state(),
      &FunctionMap::default(),
    );

    assert_eq!(result.len(), 4);

    let mut expected_result = IndexMap::new();

    expected_result.insert(
      "marginTop".to_string(),
      pre_rule_vec_factory("marginTop", &["10vh", "10dvh"], &["marginTop"]),
    );
    expected_result.insert(
      "marginInlineEnd".to_string(),
      pre_rule_factory("marginInlineEnd", "20px", &["marginInlineEnd"]),
    );
    expected_result.insert(
      "marginBottom".to_string(),
      pre_rule_vec_factory("marginBottom", &["10vh", "10dvh"], &["marginBottom"]),
    );
    expected_result.insert(
      "marginInlineStart".to_string(),
      pre_rule_factory("marginInlineStart", "20px", &["marginInlineStart"]),
    );

    assert_eq!(result, expected_result)
  }
}

#[cfg(test)]
mod nested_objects {
  use indexmap::IndexMap;

  use stylex_ast::ast::factories::{
    create_key_value_prop_ident,
    create_object_expression,
    create_string_key_value_prop,
  };
  use stylex_state::functions::FunctionMap;
  use stylex_evaluator::state::EvaluationState;
  use crate::shared::structures::tests::flatten_raw_style_objects_test::flatten_style_object_with_legacy_shorthand_expansion::{
    get_state,
    null_rule_factory,
    pre_rule_factory,
    pre_rule_set_factory,
  };
  use crate::shared::utils::core::flatten_raw_style_object::flatten_raw_style_object;
  use stylex_ast::ast::convertors::create_string_expr;

  #[test]
  fn legacy_pseudo_classes() {
    let result = flatten_raw_style_object(
      &[
        create_key_value_prop_ident("color", create_string_expr("blue")),
        create_key_value_prop_ident("marginStart", create_string_expr("0")),
        create_key_value_prop_ident(
          ":hover",
          create_object_expression(vec![
            create_string_key_value_prop("color", "red"),
            create_string_key_value_prop("marginStart", "10"),
          ]),
        ),
      ],
      &mut EvaluationState::new(),
      &mut get_state(),
      &FunctionMap::default(),
    );

    assert_eq!(result.len(), 8);

    let mut expected_result = IndexMap::new();

    expected_result.insert(
      "color".to_string(),
      pre_rule_factory("color", "blue", &["color"]),
    );
    expected_result.insert(
      "marginInlineStart".to_string(),
      pre_rule_factory("marginInlineStart", "0", &["marginInlineStart"]),
    );
    expected_result.insert("marginLeft".to_string(), null_rule_factory());
    expected_result.insert("marginRight".to_string(), null_rule_factory());
    expected_result.insert(
      ":hover_color".to_string(),
      pre_rule_factory("color", "red", &[":hover", "color"]),
    );

    expected_result.insert(
      ":hover_marginInlineStart".to_string(),
      pre_rule_factory("marginInlineStart", "10", &[":hover", "marginInlineStart"]),
    );

    expected_result.insert(":hover_marginLeft".to_string(), null_rule_factory());

    expected_result.insert(":hover_marginRight".to_string(), null_rule_factory());

    assert_eq!(result, expected_result)
  }

  #[test]
  fn modern_pseudo_classes() {
    let result = flatten_raw_style_object(
      &[
        create_key_value_prop_ident(
          "color",
          create_object_expression(vec![
            create_string_key_value_prop("default", "blue"),
            create_string_key_value_prop(":hover", "red"),
          ]),
        ),
        create_key_value_prop_ident(
          "marginStart",
          create_object_expression(vec![
            create_string_key_value_prop("default", "0"),
            create_string_key_value_prop(":hover", "10"),
          ]),
        ),
      ],
      &mut EvaluationState::new(),
      &mut get_state(),
      &FunctionMap::default(),
    );

    assert_eq!(result.len(), 4);

    let mut expected_result = IndexMap::new();

    expected_result.insert(
      "color".to_string(),
      pre_rule_set_factory(&[
        pre_rule_factory("color", "blue", &["color", "default"]),
        pre_rule_factory("color", "red", &["color", ":hover"]),
      ]),
    );

    expected_result.insert(
      "marginInlineStart".to_string(),
      pre_rule_set_factory(&[
        pre_rule_factory("marginInlineStart", "0", &["marginInlineStart", "default"]),
        pre_rule_factory("marginInlineStart", "10", &["marginInlineStart", ":hover"]),
      ]),
    );

    expected_result.insert(
      "marginLeft".to_string(),
      pre_rule_set_factory(&[null_rule_factory(), null_rule_factory()]),
    );

    expected_result.insert(
      "marginRight".to_string(),
      pre_rule_set_factory(&[null_rule_factory(), null_rule_factory()]),
    );

    assert_eq!(result, expected_result)
  }

  #[test]
  fn modern_pseudo_classes_with_shorthands() {
    let result = flatten_raw_style_object(
      &[
        create_key_value_prop_ident(
          "color",
          create_object_expression(vec![
            create_string_key_value_prop("default", "blue"),
            create_string_key_value_prop(":hover", "red"),
          ]),
        ),
        create_key_value_prop_ident(
          "margin",
          create_object_expression(vec![
            create_string_key_value_prop("default", "0"),
            create_string_key_value_prop(":hover", "10"),
          ]),
        ),
      ],
      &mut EvaluationState::new(),
      &mut get_state(),
      &FunctionMap::default(),
    );

    assert_eq!(result.len(), 5);

    let mut expected_result = IndexMap::new();

    expected_result.insert(
      "color".to_string(),
      pre_rule_set_factory(&[
        pre_rule_factory("color", "blue", &["color", "default"]),
        pre_rule_factory("color", "red", &["color", ":hover"]),
      ]),
    );

    expected_result.insert(
      "marginTop".to_string(),
      pre_rule_set_factory(&[
        pre_rule_factory("marginTop", "0", &["marginTop", "default"]),
        pre_rule_factory("marginTop", "10", &["marginTop", ":hover"]),
      ]),
    );

    expected_result.insert(
      "marginInlineEnd".to_string(),
      pre_rule_set_factory(&[
        pre_rule_factory("marginInlineEnd", "0", &["marginInlineEnd", "default"]),
        pre_rule_factory("marginInlineEnd", "10", &["marginInlineEnd", ":hover"]),
      ]),
    );

    expected_result.insert(
      "marginBottom".to_string(),
      pre_rule_set_factory(&[
        pre_rule_factory("marginBottom", "0", &["marginBottom", "default"]),
        pre_rule_factory("marginBottom", "10", &["marginBottom", ":hover"]),
      ]),
    );

    expected_result.insert(
      "marginInlineStart".to_string(),
      pre_rule_set_factory(&[
        pre_rule_factory("marginInlineStart", "0", &["marginInlineStart", "default"]),
        pre_rule_factory("marginInlineStart", "10", &["marginInlineStart", ":hover"]),
      ]),
    );

    assert_eq!(result, expected_result)
  }

  #[test]
  fn modern_pseudo_classes_with_complex_shorthands() {
    let result = flatten_raw_style_object(
      &[
        create_key_value_prop_ident(
          "color",
          create_object_expression(vec![
            create_string_key_value_prop("default", "blue"),
            create_string_key_value_prop(":hover", "red"),
          ]),
        ),
        create_key_value_prop_ident(
          "margin",
          create_object_expression(vec![
            create_string_key_value_prop("default", "1px 2px 3px 4px"),
            create_string_key_value_prop(":hover", "10px 20px"),
          ]),
        ),
      ],
      &mut EvaluationState::new(),
      &mut get_state(),
      &FunctionMap::default(),
    );

    assert_eq!(result.len(), 5);

    let mut expected_result = IndexMap::new();

    expected_result.insert(
      "color".to_string(),
      pre_rule_set_factory(&[
        pre_rule_factory("color", "blue", &["color", "default"]),
        pre_rule_factory("color", "red", &["color", ":hover"]),
      ]),
    );

    expected_result.insert(
      "marginTop".to_string(),
      pre_rule_set_factory(&[
        pre_rule_factory("marginTop", "1px", &["marginTop", "default"]),
        pre_rule_factory("marginTop", "10px", &["marginTop", ":hover"]),
      ]),
    );

    expected_result.insert(
      "marginInlineEnd".to_string(),
      pre_rule_set_factory(&[
        pre_rule_factory("marginInlineEnd", "2px", &["marginInlineEnd", "default"]),
        pre_rule_factory("marginInlineEnd", "20px", &["marginInlineEnd", ":hover"]),
      ]),
    );

    expected_result.insert(
      "marginBottom".to_string(),
      pre_rule_set_factory(&[
        pre_rule_factory("marginBottom", "3px", &["marginBottom", "default"]),
        pre_rule_factory("marginBottom", "10px", &["marginBottom", ":hover"]),
      ]),
    );

    expected_result.insert(
      "marginInlineStart".to_string(),
      pre_rule_set_factory(&[
        pre_rule_factory(
          "marginInlineStart",
          "4px",
          &["marginInlineStart", "default"],
        ),
        pre_rule_factory(
          "marginInlineStart",
          "20px",
          &["marginInlineStart", ":hover"],
        ),
      ]),
    );

    assert_eq!(result, expected_result)
  }

  #[test]
  fn modern_pseudo_and_at_rules() {
    let result = flatten_raw_style_object(
      &[
        create_key_value_prop_ident(
          "color",
          create_object_expression(vec![
            create_string_key_value_prop("default", "blue"),
            create_string_key_value_prop(":hover", "red"),
            create_string_key_value_prop("@media (min-width: 300px)", "green"),
          ]),
        ),
        create_key_value_prop_ident(
          "marginStart",
          create_object_expression(vec![
            create_string_key_value_prop("default", "0"),
            create_string_key_value_prop(":hover", "10"),
          ]),
        ),
      ],
      &mut EvaluationState::new(),
      &mut get_state(),
      &FunctionMap::default(),
    );

    assert_eq!(result.len(), 4);

    let mut expected_result = IndexMap::new();

    expected_result.insert(
      "color".to_string(),
      pre_rule_set_factory(&[
        pre_rule_factory("color", "blue", &["color", "default"]),
        pre_rule_factory("color", "red", &["color", ":hover"]),
        pre_rule_factory("color", "green", &["color", "@media (min-width: 300px)"]),
      ]),
    );

    expected_result.insert(
      "marginInlineStart".to_string(),
      pre_rule_set_factory(&[
        pre_rule_factory("marginInlineStart", "0", &["marginInlineStart", "default"]),
        pre_rule_factory("marginInlineStart", "10", &["marginInlineStart", ":hover"]),
      ]),
    );
    expected_result.insert(
      "marginLeft".to_string(),
      pre_rule_set_factory(&[null_rule_factory(), null_rule_factory()]),
    );

    expected_result.insert(
      "marginRight".to_string(),
      pre_rule_set_factory(&[null_rule_factory(), null_rule_factory()]),
    );

    assert_eq!(result, expected_result)
  }

  #[test]
  fn attribute_selector_conditions() {
    let result = flatten_raw_style_object(
      &[create_key_value_prop_ident(
        "color",
        create_object_expression(vec![
          create_string_key_value_prop("default", "blue"),
          create_string_key_value_prop("[data-panel-state=\"open\"]", "red"),
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
        pre_rule_factory("color", "red", &["color", "[data-panel-state=\"open\"]"]),
      ]),
    );

    assert_eq!(result, expected_result)
  }
}

#[cfg(test)]
mod multiple_levels_of_nesting {
  use indexmap::IndexMap;

  use stylex_ast::ast::factories::{
    create_key_value_prop_ident,
    create_nested_object_prop,
    create_object_expression,
    create_string_array_prop,
    create_string_key_value_prop,
  };
  use stylex_state::functions::FunctionMap;
  use stylex_evaluator::state::EvaluationState;
  use crate::shared::structures::tests::flatten_raw_style_objects_test::flatten_style_object_with_legacy_shorthand_expansion::{
    get_state,
    pre_rule_factory,
    pre_rule_set_factory,
    pre_rule_vec_factory,
  };
  use crate::shared::utils::core::flatten_raw_style_object::flatten_raw_style_object;

  #[test]
  fn fallback_styles_within_nested_objects() {
    let result = flatten_raw_style_object(
      &[create_key_value_prop_ident(
        "margin",
        create_object_expression(vec![
          create_string_key_value_prop("default", "1px 2px 3px 4px"),
          create_string_array_prop(":hover", &["10px 20px", "1dvh 2dvh"]),
        ]),
      )],
      &mut EvaluationState::new(),
      &mut get_state(),
      &FunctionMap::default(),
    );

    assert_eq!(result.len(), 4);

    let mut expected_result = IndexMap::new();

    expected_result.insert(
      "marginTop".to_string(),
      pre_rule_set_factory(&[
        pre_rule_factory("marginTop", "1px", &["marginTop", "default"]),
        pre_rule_vec_factory("marginTop", &["10px", "1dvh"], &["marginTop", ":hover"]),
      ]),
    );

    expected_result.insert(
      "marginInlineEnd".to_string(),
      pre_rule_set_factory(&[
        pre_rule_factory("marginInlineEnd", "2px", &["marginInlineEnd", "default"]),
        pre_rule_vec_factory(
          "marginInlineEnd",
          &["20px", "2dvh"],
          &["marginInlineEnd", ":hover"],
        ),
      ]),
    );

    expected_result.insert(
      "marginBottom".to_string(),
      pre_rule_set_factory(&[
        pre_rule_factory("marginBottom", "3px", &["marginBottom", "default"]),
        pre_rule_vec_factory(
          "marginBottom",
          &["10px", "1dvh"],
          &["marginBottom", ":hover"],
        ),
      ]),
    );

    expected_result.insert(
      "marginInlineStart".to_string(),
      pre_rule_set_factory(&[
        pre_rule_factory(
          "marginInlineStart",
          "4px",
          &["marginInlineStart", "default"],
        ),
        pre_rule_vec_factory(
          "marginInlineStart",
          &["20px", "2dvh"],
          &["marginInlineStart", ":hover"],
        ),
      ]),
    );

    assert_eq!(result, expected_result)
  }

  #[test]
  fn pseudo_within_a_media_query_legacy_syntax() {
    let result = flatten_raw_style_object(
      &[create_key_value_prop_ident(
        "@media (min-width: 300px)",
        create_object_expression(vec![create_nested_object_prop(
          ":hover",
          vec![create_string_key_value_prop("color", "red")],
        )]),
      )],
      &mut EvaluationState::new(),
      &mut get_state(),
      &FunctionMap::default(),
    );

    assert_eq!(result.len(), 1);

    let mut expected_result = IndexMap::new();

    expected_result.insert(
      "@media (min-width: 300px)_:hover_color".to_string(),
      pre_rule_set_factory(&[pre_rule_factory(
        "color",
        "red",
        &["@media (min-width: 300px)", ":hover", "color"],
      )]),
    );

    assert_eq!(result, expected_result)
  }

  #[test]
  fn pseudo_with_a_pseudo_within_a_media_query_legacy_syntax() {
    let result = flatten_raw_style_object(
      &[create_key_value_prop_ident(
        "@media (min-width: 300px)",
        create_object_expression(vec![create_nested_object_prop(
          ":hover",
          vec![
            create_string_key_value_prop("color", "pink"),
            create_nested_object_prop(
              ":active",
              vec![create_string_key_value_prop("color", "red")],
            ),
          ],
        )]),
      )],
      &mut EvaluationState::new(),
      &mut get_state(),
      &FunctionMap::default(),
    );

    assert_eq!(result.len(), 2);

    let mut expected_result = IndexMap::new();

    expected_result.insert(
      "@media (min-width: 300px)_:hover_color".to_string(),
      pre_rule_set_factory(&[pre_rule_factory(
        "color",
        "pink",
        &["@media (min-width: 300px)", ":hover", "color"],
      )]),
    );

    expected_result.insert(
      "@media (min-width: 300px)_:hover_:active_color".to_string(),
      pre_rule_set_factory(&[pre_rule_factory(
        "color",
        "red",
        &["@media (min-width: 300px)", ":hover", ":active", "color"],
      )]),
    );

    assert_eq!(result, expected_result)
  }

  #[test]
  fn pseudo_within_a_media_query_modern_syntax() {
    let result = flatten_raw_style_object(
      &[create_key_value_prop_ident(
        "color",
        create_object_expression(vec![
          create_string_key_value_prop("default", "blue"),
          create_nested_object_prop(
            "@media (min-width: 300px)",
            vec![create_string_key_value_prop(":hover", "red")],
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
          &["color", "@media (min-width: 300px)", ":hover"],
        ),
      ]),
    );

    assert_eq!(result, expected_result)
  }

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
