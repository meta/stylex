#[cfg(test)]
mod stylex_define_vars_nested {
  use crate::{
    shared::transformers::stylex_define_vars_nested::stylex_define_vars_nested,
    shared::utils::core::stylex_nested_utils::UnflattenedCompiledStylesValue,
  };
  use stylex_ast::ast::convertors::create_string_expr;
  use stylex_ast::ast::factories::{create_key_value_prop, create_object_expression};
  use stylex_state::{evaluate_result_value::EvaluateResultValue, state_manager::StateManager};
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

  fn as_string_leaf(value: &UnflattenedCompiledStylesValue) -> &str {
    let UnflattenedCompiledStylesValue::Leaf(value) = value else {
      panic!("expected leaf")
    };
    let Some(value) = value.as_string() else {
      panic!("expected string leaf")
    };
    value
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
}
