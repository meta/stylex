//! Current Babel `positionTry` contracts: one declaration per resolved key.

#[cfg(test)]
mod stylex_position_try {
  use crate::shared::transformers::stylex_position_try::stylex_position_try;
  use crate::tests::support::expr;
  use stylex_enums::style_resolution::StyleResolution;
  use stylex_state::{evaluate_result_value::EvaluateResultValue, state_manager::StateManager};
  use stylex_types::{
    enums::data_structures::injectable_style::InjectableStyleKind,
    structures::injectable_style::InjectableStyle,
  };

  fn compile_with_state(code: &str, state: &mut StateManager) -> (String, InjectableStyle) {
    let (name, rule) = stylex_position_try(&EvaluateResultValue::Expr(expr(code)), state);
    match rule {
      InjectableStyleKind::Regular(style) => (name, style),
      other => panic!("a fallback is a regular rule: {other:?}"),
    }
  }

  fn compile(code: &str) -> (String, InjectableStyle) {
    compile_with_state(code, &mut StateManager::default())
  }

  #[test]
  fn writes_each_declaration_once() {
    let (name, style) = compile("{ top: '10px', marginTop: '4px' }");
    assert_eq!(
      style.ltr,
      format!("@position-try {name} {{margin-top:4px;top:10px;}}")
    );
  }

  #[test]
  fn identical_directional_declarations_have_no_rtl_rule() {
    let (name, style) = compile("{ top: '10px' }");
    assert_eq!(style.ltr, format!("@position-try {name} {{top:10px;}}"));
    assert_eq!(style.rtl, None);
  }

  #[test]
  fn configured_logical_key_resolves_to_each_physical_direction() {
    let mut state = StateManager::default();
    state.options.style_resolution = StyleResolution::LegacyExpandShorthands;
    state.options.enable_logical_styles_polyfill = true;
    let (name, style) = compile_with_state("{ insetInlineStart: 8 }", &mut state);
    assert_eq!(style.ltr, format!("@position-try {name} {{left:8px;}}"));
    assert_eq!(
      style.rtl,
      Some(format!("@position-try {name} {{right:8px;}}"))
    );
  }

  #[test]
  fn writes_an_empty_body_for_a_fallback_that_declares_nothing() {
    let (name, style) = compile("{}");
    assert_eq!(style.ltr, format!("@position-try {name} {{}}"));
    assert_eq!(style.rtl, None);
  }

  #[test]
  fn sorts_the_declarations_by_resolved_property_name() {
    let (name, style) = compile("{ top: '1px', bottom: '2px', left: '3px' }");
    assert_eq!(
      style.ltr,
      format!("@position-try {name} {{bottom:2px;left:3px;top:1px;}}")
    );
  }
}
