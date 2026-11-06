#[cfg(test)]
mod converting_pre_rule_to_css {
  use indexmap::IndexMap;
  use stylex_structures::pre_rule_value::PreRuleValue;
  use stylex_types::structures::style_key::ClassName;

  use crate::{
    shared::structures::pre_rule::CompiledResult, shared::structures::pre_rule::ComputedStyle,
    shared::structures::pre_rule::PreRule, shared::structures::pre_rule::StylesPreRule,
  };
  use stylex_enums::style_resolution::StyleResolution;
  use stylex_state::{state_manager::StateManager, types::ClassNameToOriginalPaths};
  use stylex_types::structures::injectable_style::InjectableStyle;

  pub(super) fn get_state() -> StateManager {
    let mut state_manager = StateManager::default();

    state_manager.options = state_manager
      .options
      .with_class_name_prefix("x")
      .with_dev(false)
      .with_debug(false)
      .with_enable_dev_class_names(false)
      .with_enable_debug_data_prop(true)
      .with_enable_font_size_px_to_rem(false)
      .with_enable_logical_styles_polyfill(false)
      .with_enable_minified_keys(true)
      .with_style_resolution(StyleResolution::LegacyExpandShorthands)
      .with_test(false);

    state_manager
  }

  #[test]
  fn should_convert_a_pre_rule_to_css() {
    let result = StylesPreRule::new(
      "color",
      PreRuleValue::string("red"),
      Some(vec!["color".to_string()]),
    )
    .compiled(&mut get_state());

    let mut classes_to_original_paths: ClassNameToOriginalPaths = IndexMap::new();
    classes_to_original_paths.insert(ClassName::from("x1e2nbdu"), vec!["color".to_string()]);

    assert_eq!(
      result,
      CompiledResult::ComputedStyles(vec![ComputedStyle(
        ClassName::from("x1e2nbdu"),
        InjectableStyle {
          ltr: ".x1e2nbdu{color:red}".to_string(),
          rtl: None,
          priority: Some(3000.0)
        },
        classes_to_original_paths
      )])
    );
  }
}
