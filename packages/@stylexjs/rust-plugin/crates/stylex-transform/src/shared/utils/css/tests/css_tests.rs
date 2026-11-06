#[cfg(test)]
mod common_css_tests {
  use crate::shared::utils::css::common::transform_value_cached;
  use stylex_css::css::common::get_number_suffix;
  use stylex_state::state_manager::StateManager;
  use stylex_structures::raw_value::TRawValue;

  #[test]
  fn should_transform_css_property_value() {
    // Only a number takes the property's unit suffix; a numeric-looking string
    // is emitted as authored.
    assert_eq!(
      transform_value_cached(
        "padding",
        &TRawValue::from(1.0),
        &mut StateManager::default()
      ),
      "1px"
    );
    assert_eq!(
      transform_value_cached(
        "padding",
        &TRawValue::from("1"),
        &mut StateManager::default()
      ),
      "1"
    );
  }

  #[test]
  fn should_return_correct_suffix() {
    assert_eq!(get_number_suffix("padding"), "px");
    assert_eq!(get_number_suffix("opacity"), "");
    assert_eq!(get_number_suffix("voiceDuration"), "ms");
  }
}
