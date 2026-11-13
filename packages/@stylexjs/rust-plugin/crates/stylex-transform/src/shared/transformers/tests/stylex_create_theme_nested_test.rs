#[cfg(test)]
mod stylex_create_theme_nested {
  use indexmap::IndexMap;

  use crate::shared::transformers::stylex_create_theme_nested::stylex_create_theme_nested;
  use stylex_state::{evaluate_result_value::EvaluateResultValue, state_manager::StateManager};

  /// The group answers a name for any key, so it is handed on as is.
  #[test]
  fn overrides_the_group_a_variable_defining_module_exports() {
    use stylex_state::theme_ref::ThemeRef;

    use crate::tests::support::expr;

    let theme_vars =
      EvaluateResultValue::ThemeRef(ThemeRef::new("tokens.stylex.js", "tokens", "x"));

    let (class_name_output, css_output) = stylex_create_theme_nested(
      &theme_vars,
      &EvaluateResultValue::Expr(expr("{ button: { bgColor: 'green' } }")),
      &mut StateManager::default(),
      &mut IndexMap::default(),
    );

    assert_eq!(css_output.len(), 1);
    assert_eq!(class_name_output.len(), 2);
  }
}
