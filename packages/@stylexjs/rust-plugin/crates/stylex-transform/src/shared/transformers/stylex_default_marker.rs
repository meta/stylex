use std::rc::Rc;

use indexmap::IndexMap;

use stylex_constants::constants::common::COMPILED_KEY;
use stylex_state::{
  flat_compiled_styles_value::FlatCompiledStylesValue, state_manager::StateManager,
  types::FlatCompiledStyles,
};
use stylex_structures::stylex_state_options::StyleXStateOptions;

/// The default marker's entries: the marker class name as key and value, plus `$$css: true`.
fn default_marker_values(options: &StyleXStateOptions) -> FlatCompiledStyles {
  // `classNamePrefix` is already defaulted to `x`, so an empty prefix is explicit and keeps `-`.
  let prefix = format!("{}-", options.class_name_prefix);

  let marker_class = format!("{}default-marker", prefix);

  let mut result = IndexMap::new();

  result.insert(
    marker_class.clone(),
    Rc::new(FlatCompiledStylesValue::String(marker_class)),
  );

  result.insert(
    COMPILED_KEY.to_string(),
    Rc::new(FlatCompiledStylesValue::Bool(true)),
  );

  result
}

/// This file's default marker, built on first use and shared after.
/// See [`StateManager::cached_default_marker_values`].
pub(crate) fn shared_default_marker_values(state: &mut StateManager) -> Rc<FlatCompiledStyles> {
  if let Some(values) = state.cached_default_marker_values() {
    return Rc::clone(values);
  }

  let values = Rc::new(default_marker_values(&state.options));

  state.insert_cached_default_marker_values(Rc::clone(&values));

  values
}

#[cfg(test)]
#[path = "../../tests/stylex_default_marker_tests.rs"]
mod tests;
