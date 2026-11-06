#[cfg(test)]
mod convert_style_to_class_name {
  use crate::shared::utils::core::convert_style_to_class_name::convert_style_to_class_name;

  use stylex_state::state_manager::StateManager;
  use stylex_structures::pre_rule_value::PreRuleValue;

  fn convert(styles: (&str, &PreRuleValue)) -> String {
    match try_convert(styles) {
      Some(declaration) => declaration,
      None => panic!("expected `{}` to compile to a declaration", styles.0),
    }
  }

  /// `None` when the pair carries no CSS text and is left undeclared.
  fn try_convert(styles: (&str, &PreRuleValue)) -> Option<String> {
    convert_style_to_class_name(
      styles,
      &mut [],
      &mut [],
      &mut [],
      &mut StateManager::default(),
    )
    .map(|(_, _, rule)| extract_body(rule.ltr))
  }

  fn extract_body(s: String) -> String {
    let start = s.find('{').unwrap_or(0) + 1;
    let end = s.len() - 1;
    s[start..end].to_string()
  }

  /// The reference implementation crashes in its value parser here, so there is no answer to match.
  #[test]
  fn declares_nothing_for_a_control_character() {
    assert_eq!(try_convert(("color", &PreRuleValue::string("\u{1}"))), None);
  }

  #[test]
  #[should_panic(expected = "A style value can only contain an array, string or number.")]
  fn refuses_an_absent_value() {
    convert(("color", &PreRuleValue::Null));
  }
}
