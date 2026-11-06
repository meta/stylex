use std::rc::Rc;

use indexmap::IndexMap;

use stylex_constants::constants::common::COMPILED_KEY;
use stylex_state::{
  flat_compiled_styles_value::FlatCompiledStylesValue, state_manager::StateManager,
  types::FlatCompiledStyles,
};
use stylex_structures::stylex_state_options::StyleXStateOptions;

/// The default marker's entries: the marker class name as both key and value,
/// and the `$$css` marker set to true.
///
/// A `stylex.props` call carrying this marker adds the class an ancestor or
/// sibling state observer looks for.
fn default_marker_values(options: &StyleXStateOptions) -> FlatCompiledStyles {
  // NOTE: the prefix is always applied, including when it is empty — an
  // unset `classNamePrefix` arrives here already defaulted to `x`, so an
  // empty one was asked for explicitly and keeps its separator.
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

/// This file's default marker, built on the first call and shared after it.
///
/// A `create` call, every call of the `props` family and `defaultMarker()`
/// itself all ask for it, and the first two register it again under every name
/// the file imports it by. See [`StateManager::cached_default_marker_values`]
/// for why one answer serves them all.
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
