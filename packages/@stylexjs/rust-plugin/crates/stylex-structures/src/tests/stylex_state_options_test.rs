//! Tests for StyleXStateOptions default values and From<StyleXOptions>
//! conversion.

use crate::{
  named_import_source::{RuntimeInjection, RuntimeInjectionState},
  stylex_options::StyleXOptions,
  stylex_state_options::*,
};

#[test]
fn from_stylex_options_boolean_true_injection() {
  let opts = StyleXOptions {
    runtime_injection: RuntimeInjection::Boolean(true),
    ..StyleXOptions::default()
  };
  let state: StyleXStateOptions = opts.into();
  assert!(matches!(
    state.runtime_injection,
    Some(RuntimeInjectionState::Regular(_))
  ));
}
