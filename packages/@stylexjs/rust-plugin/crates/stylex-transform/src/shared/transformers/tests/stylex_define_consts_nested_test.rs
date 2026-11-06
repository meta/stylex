#[cfg(test)]
mod stylex_define_consts_nested {
  use stylex_ast::ast::convertors::{create_number_expr, create_string_expr};
  use stylex_ast::ast::factories::{create_key_value_prop, create_object_expression};
  use stylex_structures::stylex_options::StyleXOptions;
  use stylex_types::enums::data_structures::injectable_style::InjectableStyleKind;

  use crate::{
    shared::transformers::stylex_define_consts::stylex_define_consts,
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

  fn consts_fixture() -> EvaluateResultValue {
    EvaluateResultValue::Expr(create_object_expression(vec![create_key_value_prop(
      "spacing",
      create_object_expression(vec![
        create_key_value_prop("sm", create_number_expr(4.0)),
        create_key_value_prop("md", create_string_expr("8px")),
      ]),
    )]))
  }

  #[test]
  fn returns_nested_js_output_with_original_values_preserved() {
    let mut state = create_test_state_manager();

    let (js_output, styles) = stylex_define_consts_nested(&consts_fixture(), &mut state);

    let Some(UnflattenedCompiledStylesValue::Object(spacing)) = js_output.get("spacing") else {
      panic!("expected nested spacing object")
    };
    assert!(matches!(
      spacing["sm"],
      UnflattenedCompiledStylesValue::Leaf(_)
    ));
    assert!(matches!(
      spacing["md"],
      UnflattenedCompiledStylesValue::Leaf(_)
    ));

    assert_eq!(styles.len(), 2);
    assert!(styles.values().all(|style| match style.as_ref() {
      InjectableStyleKind::Const(style) => style.ltr.is_empty(),
      InjectableStyleKind::Regular(_) => false,
    }));
  }

  #[test]
  fn keeps_injectable_const_metadata_flat() {
    let mut state = create_test_state_manager();

    let (_, styles) = stylex_define_consts_nested(&consts_fixture(), &mut state);
    let const_values = styles
      .values()
      .filter_map(|style| match style.as_ref() {
        InjectableStyleKind::Const(style) => Some(style.const_value.as_str()),
        InjectableStyleKind::Regular(_) => None,
      })
      .collect::<Vec<_>>();

    assert_eq!(styles.len(), 2);
    assert!(const_values.contains(&"4"));
    assert!(const_values.contains(&"\"8px\""));
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

  fn three_tiered_fixture() -> EvaluateResultValue {
    EvaluateResultValue::Expr(create_object_expression(vec![create_key_value_prop(
      "button",
      create_object_expression(vec![create_key_value_prop(
        "primary",
        create_object_expression(vec![
          create_key_value_prop(
            "background",
            create_object_expression(vec![
              create_key_value_prop("default", create_string_expr("#00FF00")),
              create_key_value_prop("hovered", create_string_expr("#0000FF")),
            ]),
          ),
          create_key_value_prop(
            "borderRadius",
            create_object_expression(vec![create_key_value_prop(
              "default",
              create_string_expr("8px"),
            )]),
          ),
        ]),
      )]),
    )]))
  }

  #[test]
  fn j_malt_pr_1303_use_case_three_tiered_design_system_tokens_with_state_namespaces() {
    let mut state = create_test_state_manager();
    let (js_output, styles) = stylex_define_consts_nested(&three_tiered_fixture(), &mut state);

    let Some(UnflattenedCompiledStylesValue::Object(button)) = js_output.get("button") else {
      panic!("expected button namespace")
    };
    let Some(UnflattenedCompiledStylesValue::Object(primary)) = button.get("primary") else {
      panic!("expected primary namespace")
    };
    let Some(UnflattenedCompiledStylesValue::Object(bg)) = primary.get("background") else {
      panic!("expected background namespace (state namespace)")
    };
    assert!(matches!(
      bg["default"],
      UnflattenedCompiledStylesValue::Leaf(_)
    ));
    assert!(matches!(
      bg["hovered"],
      UnflattenedCompiledStylesValue::Leaf(_)
    ));
    assert_eq!(styles.len(), 3);
  }

  fn number_values_fixture() -> EvaluateResultValue {
    EvaluateResultValue::Expr(create_object_expression(vec![create_key_value_prop(
      "breakpoints",
      create_object_expression(vec![
        create_key_value_prop("mobile", create_number_expr(480.0)),
        create_key_value_prop("tablet", create_number_expr(768.0)),
      ]),
    )]))
  }

  #[test]
  fn preserves_number_values() {
    let mut state = create_test_state_manager();
    let (js_output, styles) = stylex_define_consts_nested(&number_values_fixture(), &mut state);
    let Some(UnflattenedCompiledStylesValue::Object(breakpoints)) = js_output.get("breakpoints")
    else {
      panic!("expected breakpoints namespace")
    };
    assert!(matches!(
      breakpoints["mobile"],
      UnflattenedCompiledStylesValue::Leaf(_)
    ));
    assert!(matches!(
      breakpoints["tablet"],
      UnflattenedCompiledStylesValue::Leaf(_)
    ));
    let const_values = styles
      .values()
      .filter_map(|style| match style.as_ref() {
        InjectableStyleKind::Const(style) => Some(style.const_value.as_str()),
        InjectableStyleKind::Regular(_) => None,
      })
      .collect::<Vec<_>>();
    // The rule carries the constant as JSON, so a number stays a number all
    // the way to the reader of the metadata.
    assert!(const_values.contains(&"480"));
    assert!(const_values.contains(&"768"));
  }

  fn mixed_string_number_fixture() -> EvaluateResultValue {
    EvaluateResultValue::Expr(create_object_expression(vec![create_key_value_prop(
      "theme",
      create_object_expression(vec![
        create_key_value_prop("spacing", create_number_expr(8.0)),
        create_key_value_prop("unit", create_string_expr("px")),
      ]),
    )]))
  }

  #[test]
  fn handles_mixed_string_and_number_values() {
    let mut state = create_test_state_manager();
    let (_, styles) = stylex_define_consts_nested(&mixed_string_number_fixture(), &mut state);
    let const_values = styles
      .values()
      .filter_map(|style| match style.as_ref() {
        InjectableStyleKind::Const(style) => Some(style.const_value.as_str()),
        InjectableStyleKind::Regular(_) => None,
      })
      .collect::<Vec<_>>();
    // A number stays a number and text arrives quoted, because the rule
    // carries the constant as JSON.
    assert!(const_values.contains(&"8"));
    assert!(const_values.contains(&"\"px\""));
  }

  #[test]
  fn generates_empty_css_consts_do_not_emit_css_variables() {
    let mut state = create_test_state_manager();
    let (_, styles) = stylex_define_consts_nested(&consts_fixture(), &mut state);
    let has_any_css = styles.values().any(|style| match style.as_ref() {
      InjectableStyleKind::Const(style) => !style.ltr.is_empty(),
      InjectableStyleKind::Regular(_) => true,
    });
    assert!(
      !has_any_css,
      "consts should not emit any CSS variables or rules"
    );
  }

  fn single_nested_consts_fixture() -> EvaluateResultValue {
    EvaluateResultValue::Expr(create_object_expression(vec![create_key_value_prop(
      "spacing",
      create_object_expression(vec![create_key_value_prop("sm", create_string_expr("4px"))]),
    )]))
  }

  fn equivalent_flat_consts_fixture() -> EvaluateResultValue {
    EvaluateResultValue::Expr(create_object_expression(vec![create_key_value_prop(
      "spacing.sm",
      create_string_expr("4px"),
    )]))
  }

  #[test]
  fn produces_same_const_metadata_as_flat_define_consts_with_equivalent_keys() {
    let mut nested_state = create_test_state_manager();
    let mut flat_state = create_test_state_manager();

    let (_, nested_meta) =
      stylex_define_consts_nested(&single_nested_consts_fixture(), &mut nested_state);
    let (_, flat_meta) = stylex_define_consts(&equivalent_flat_consts_fixture(), &mut flat_state);

    // Same number of entries.
    assert_eq!(nested_meta.len(), flat_meta.len());

    let nested_const_val = match nested_meta.values().next().map(|style| style.as_ref()) {
      Some(InjectableStyleKind::Const(style)) => style.const_value.clone(),
      _ => panic!("expected const style"),
    };
    let flat_const_val = match flat_meta.values().next().map(|style| style.as_ref()) {
      Some(InjectableStyleKind::Const(style)) => style.const_value.clone(),
      _ => panic!("expected const style"),
    };

    // Same constVal.
    assert_eq!(nested_const_val, flat_const_val);
  }
}
