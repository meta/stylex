#[cfg(test)]
mod stylex_define_consts_nested {
  use stylex_ast::ast::convertors::create_string_expr;
  use stylex_ast::ast::factories::{create_key_value_prop, create_object_expression};
  use stylex_structures::stylex_options::StyleXOptions;

  use crate::{
    shared::transformers::stylex_define_consts_nested::stylex_define_consts_nested,
    shared::utils::core::stylex_nested_utils::UnflattenedCompiledStylesValue,
  };
  use stylex_state::{evaluate_result_value::EvaluateResultValue, state_manager::StateManager};

  fn create_test_state_manager() -> StateManager {
    let options = StyleXOptions::default()
      .with_class_name_prefix("x")
      .with_debug(false);
    let mut state = StateManager::new(options);
    state.export_id = Some("test/tokens.stylex.js//tokens".to_string());
    state
  }

  fn deeply_nested_consts_fixture() -> EvaluateResultValue {
    EvaluateResultValue::Expr(create_object_expression(vec![create_key_value_prop(
      "colors",
      create_object_expression(vec![
        create_key_value_prop(
          "slate",
          create_object_expression(vec![
            create_key_value_prop("light", create_string_expr("#f1f5f9")),
            create_key_value_prop("dark", create_string_expr("#1e293b")),
          ]),
        ),
        create_key_value_prop(
          "brand",
          create_object_expression(vec![create_key_value_prop(
            "primary",
            create_string_expr("#3b82f6"),
          )]),
        ),
      ]),
    )]))
  }

  #[test]
  fn handles_deeply_nested_constants() {
    let mut state = create_test_state_manager();
    let (js_output, _) = stylex_define_consts_nested(&deeply_nested_consts_fixture(), &mut state);

    let Some(UnflattenedCompiledStylesValue::Object(colors)) = js_output.get("colors") else {
      panic!("expected colors namespace")
    };
    let Some(UnflattenedCompiledStylesValue::Object(slate)) = colors.get("slate") else {
      panic!("expected slate namespace")
    };
    assert!(matches!(
      slate["light"],
      UnflattenedCompiledStylesValue::Leaf(_)
    ));
    assert!(matches!(
      slate["dark"],
      UnflattenedCompiledStylesValue::Leaf(_)
    ));
    let Some(UnflattenedCompiledStylesValue::Object(brand)) = colors.get("brand") else {
      panic!("expected brand namespace")
    };
    assert!(matches!(
      brand["primary"],
      UnflattenedCompiledStylesValue::Leaf(_)
    ));
  }
}
