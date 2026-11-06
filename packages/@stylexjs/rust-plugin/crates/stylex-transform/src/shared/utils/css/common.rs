use stylex_css::css::common::{get_number_suffix, normalize_css_property_value};
use stylex_css::values::content::transform_content_value;
use stylex_state::state_manager::StateManager;
use stylex_structures::raw_value::TRawValue;
use stylex_utils::{math::round_f64, number::to_js_string, string::is_blank_css_text};

/// Converts a CSS value in JS to the final CSS string value.
/// Only numbers get a unit suffix; `gridTemplateColumns: '1'` stays `1`.
pub(crate) fn transform_value(key: &str, raw_value: &TRawValue, state: &StateManager) -> String {
  let value = match raw_value.as_number() {
    Some(number) => {
      let mut value = to_js_string(round_f64(number, 4));
      value.push_str(get_number_suffix(key));
      value
    },
    None => raw_value.as_css_text().into_owned(),
  };

  if key == "content" || key == "hyphenateCharacter" || key == "hyphenate-character" {
    return transform_content_value(value.trim());
  }

  // Blank values skip normalization; `convert_style_to_class_name` then emits no declaration.
  if is_blank_css_text(&value) {
    return value;
  }

  normalize_css_property_value(key, value.as_ref(), &state.options)
}

pub(crate) fn transform_value_cached(
  key: &str,
  value: &TRawValue,
  state: &mut StateManager,
) -> String {
  // Keyed by JS identity, not CSS text: `width: 1` and `width: '1'` compile differently.
  let identity_len = match value.as_number() {
    // A tag and room for a JS number's spelling.
    Some(_) => 1 + 24,
    None => 1 + value.as_css_text().len(),
  };
  let mut cache_key = String::with_capacity(key.len() + 1 + identity_len);
  cache_key.push_str(key);
  cache_key.push(':');
  value.write_identity_key(&mut cache_key);

  let cache = state.css_property_seen().get(&cache_key);

  if let Some(result) = cache {
    return result.to_string();
  }

  let result = transform_value(key, value, state);

  state
    .css_property_seen_mut()
    .insert(cache_key, result.clone());

  result
}
