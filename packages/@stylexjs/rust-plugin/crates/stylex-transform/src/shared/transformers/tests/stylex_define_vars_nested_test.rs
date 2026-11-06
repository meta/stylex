#[cfg(test)]
mod stylex_define_vars_nested {
  use crate::{
    shared::transformers::stylex_define_vars::stylex_define_vars,
    shared::transformers::stylex_define_vars_nested::stylex_define_vars_nested,
    shared::utils::core::stylex_nested_utils::UnflattenedCompiledStylesValue,
  };
  use stylex_ast::ast::convertors::create_string_expr;
  use stylex_ast::ast::factories::{create_key_value_prop, create_object_expression};
  use stylex_state::{
    evaluate_result_value::EvaluateResultValue,
    flat_compiled_styles_value::FlatCompiledStylesValue, state_manager::StateManager,
  };
  use stylex_structures::stylex_options::StyleXOptions;
  use stylex_types::enums::data_structures::injectable_style::InjectableStyleKind;

  fn create_test_state_manager() -> StateManager {
    let options = StyleXOptions::default()
      .with_class_name_prefix("x")
      .with_debug(false);
    let mut state = StateManager::new(options);
    state.export_id = Some("test/tokens.stylex.js//tokens".to_string());
    state
  }

  fn nested_vars_fixture() -> EvaluateResultValue {
    EvaluateResultValue::Expr(create_object_expression(vec![create_key_value_prop(
      "button",
      create_object_expression(vec![
        create_key_value_prop("background", create_string_expr("red")),
        create_key_value_prop(
          "color",
          create_object_expression(vec![
            create_key_value_prop("default", create_string_expr("blue")),
            create_key_value_prop(
              "@media (prefers-color-scheme: dark)",
              create_string_expr("lightblue"),
            ),
          ]),
        ),
      ]),
    )]))
  }

  fn as_string_leaf(value: &UnflattenedCompiledStylesValue) -> &str {
    let UnflattenedCompiledStylesValue::Leaf(value) = value else {
      panic!("expected leaf")
    };
    let Some(value) = value.as_string() else {
      panic!("expected string leaf")
    };
    value
  }

  #[test]
  fn returns_nested_js_output_with_var_references() {
    let mut state = create_test_state_manager();

    let (js_output, css_output) = stylex_define_vars_nested(&nested_vars_fixture(), &mut state);

    let Some(UnflattenedCompiledStylesValue::Object(button)) = js_output.get("button") else {
      panic!("expected nested button object")
    };

    assert!(as_string_leaf(&button["background"]).starts_with("var(--"));
    assert!(as_string_leaf(&button["color"]).starts_with("var(--"));
    assert!(as_string_leaf(&js_output["__varGroupHash__"]).starts_with('x'));

    let all_css = css_output
      .values()
      .map(|style| match style.as_ref() {
        InjectableStyleKind::Regular(style) => style.ltr.as_str(),
        InjectableStyleKind::Const(_) => "",
      })
      .collect::<String>();

    assert!(all_css.contains("red"));
    assert!(all_css.contains("blue"));
    assert!(all_css.contains("lightblue"));
    assert!(all_css.contains("@media"));
  }

  #[test]
  fn different_nested_keys_produce_different_var_hashes() {
    let mut state = create_test_state_manager();

    let (js_output, _) = stylex_define_vars_nested(&nested_vars_fixture(), &mut state);
    let Some(UnflattenedCompiledStylesValue::Object(button)) = js_output.get("button") else {
      panic!("expected nested button object")
    };

    assert_ne!(
      as_string_leaf(&button["background"]),
      as_string_leaf(&button["color"])
    );
  }

  fn deeply_nested_fixture() -> EvaluateResultValue {
    EvaluateResultValue::Expr(create_object_expression(vec![create_key_value_prop(
      "button",
      create_object_expression(vec![
        create_key_value_prop(
          "primary",
          create_object_expression(vec![create_key_value_prop(
            "background",
            create_string_expr("#00FF00"),
          )]),
        ),
        create_key_value_prop(
          "secondary",
          create_object_expression(vec![create_key_value_prop(
            "background",
            create_string_expr("#CCCCCC"),
          )]),
        ),
      ]),
    )]))
  }

  #[test]
  fn handles_deeply_nested_tokens_3_levels() {
    let mut state = create_test_state_manager();
    let (js_output, css_output) = stylex_define_vars_nested(&deeply_nested_fixture(), &mut state);

    let Some(UnflattenedCompiledStylesValue::Object(button)) = js_output.get("button") else {
      panic!("expected button namespace")
    };
    let Some(UnflattenedCompiledStylesValue::Object(primary)) = button.get("primary") else {
      panic!("expected primary namespace")
    };
    let Some(UnflattenedCompiledStylesValue::Object(secondary)) = button.get("secondary") else {
      panic!("expected secondary namespace")
    };
    assert!(as_string_leaf(&primary["background"]).starts_with("var(--"));
    assert!(as_string_leaf(&secondary["background"]).starts_with("var(--"));

    let all_css = css_output
      .values()
      .map(|style| match style.as_ref() {
        InjectableStyleKind::Regular(style) => style.ltr.as_str(),
        InjectableStyleKind::Const(_) => "",
      })
      .collect::<String>();
    assert!(all_css.contains("#00FF00"));
    assert!(all_css.contains("#CCCCCC"));
  }

  #[test]
  fn handles_conditional_at_rule_values_inside_nesting() {
    let mut state = create_test_state_manager();
    let (_, css_output) = stylex_define_vars_nested(&nested_vars_fixture(), &mut state);
    let all_css = css_output
      .values()
      .map(|style| match style.as_ref() {
        InjectableStyleKind::Regular(style) => style.ltr.as_str(),
        InjectableStyleKind::Const(_) => "",
      })
      .collect::<String>();
    assert!(all_css.contains("@media (prefers-color-scheme: dark)"));
    assert!(all_css.contains("lightblue"));
  }

  fn mixed_flat_nested_fixture() -> EvaluateResultValue {
    EvaluateResultValue::Expr(create_object_expression(vec![
      create_key_value_prop("flatValue", create_string_expr("red")),
      create_key_value_prop(
        "nested",
        create_object_expression(vec![create_key_value_prop(
          "deep",
          create_string_expr("blue"),
        )]),
      ),
    ]))
  }

  #[test]
  fn handles_mixed_flat_and_nested_values() {
    let mut state = create_test_state_manager();
    let (js_output, _) = stylex_define_vars_nested(&mixed_flat_nested_fixture(), &mut state);

    assert!(as_string_leaf(&js_output["flatValue"]).starts_with("var(--"));
    let Some(UnflattenedCompiledStylesValue::Object(nested)) = js_output.get("nested") else {
      panic!("expected nested namespace")
    };
    assert!(as_string_leaf(&nested["deep"]).starts_with("var(--"));
  }

  #[test]
  fn includes_var_group_hash_at_top_level() {
    let mut state = create_test_state_manager();
    let (js_output, _) = stylex_define_vars_nested(&nested_vars_fixture(), &mut state);
    assert!(js_output.contains_key("__varGroupHash__"));
    assert!(as_string_leaf(&js_output["__varGroupHash__"]).starts_with('x'));
  }

  #[test]
  fn produces_css_with_var_declarations() {
    let mut state = create_test_state_manager();
    let (_, css_output) = stylex_define_vars_nested(&nested_vars_fixture(), &mut state);
    let all_css = css_output
      .values()
      .map(|style| match style.as_ref() {
        InjectableStyleKind::Regular(style) => style.ltr.as_str(),
        InjectableStyleKind::Const(_) => "",
      })
      .collect::<String>();
    assert!(all_css.contains(":root"));
    assert!(all_css.contains("--"));
  }

  fn simple_nested_vars_fixture() -> EvaluateResultValue {
    EvaluateResultValue::Expr(create_object_expression(vec![create_key_value_prop(
      "button",
      create_object_expression(vec![create_key_value_prop(
        "background",
        create_string_expr("red"),
      )]),
    )]))
  }

  fn equivalent_flat_vars_fixture() -> EvaluateResultValue {
    EvaluateResultValue::Expr(create_object_expression(vec![create_key_value_prop(
      "button.background",
      create_string_expr("red"),
    )]))
  }

  #[test]
  fn produces_same_css_as_flat_define_vars_with_equivalent_keys() {
    let mut nested_state = create_test_state_manager();
    let mut flat_state = create_test_state_manager();

    let (_, nested_css) =
      stylex_define_vars_nested(&simple_nested_vars_fixture(), &mut nested_state);
    let (_, flat_css) = stylex_define_vars(&equivalent_flat_vars_fixture(), &mut flat_state);

    // Same number of injectable style entries.
    assert_eq!(nested_css.len(), flat_css.len());

    let nested_ltr = nested_css
      .values()
      .next()
      .map(|style| match style.as_ref() {
        InjectableStyleKind::Regular(style) => style.ltr.clone(),
        InjectableStyleKind::Const(_) => String::new(),
      })
      .unwrap_or_default();
    let flat_ltr = flat_css
      .values()
      .next()
      .map(|style| match style.as_ref() {
        InjectableStyleKind::Regular(style) => style.ltr.clone(),
        InjectableStyleKind::Const(_) => String::new(),
      })
      .unwrap_or_default();

    assert!(nested_ltr.contains("red"));
    assert!(flat_ltr.contains("red"));
  }

  #[test]
  fn produces_same_var_group_hash_as_flat_define_vars() {
    let mut nested_state = create_test_state_manager();
    let mut flat_state = create_test_state_manager();

    let (nested_js, _) =
      stylex_define_vars_nested(&simple_nested_vars_fixture(), &mut nested_state);
    let (flat_js, _) = stylex_define_vars(&equivalent_flat_vars_fixture(), &mut flat_state);

    let nested_hash = as_string_leaf(&nested_js["__varGroupHash__"]).to_string();
    let flat_hash = match flat_js.get("__varGroupHash__").map(|value| value.as_ref()) {
      Some(FlatCompiledStylesValue::String(value)) => value.clone(),
      _ => panic!("expected flat __varGroupHash__ string"),
    };

    // Same group hash because same exportId.
    assert_eq!(nested_hash, flat_hash);
  }
}
