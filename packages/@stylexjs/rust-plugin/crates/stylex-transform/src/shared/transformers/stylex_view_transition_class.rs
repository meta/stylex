use std::fmt::Write;

use indexmap::IndexMap;
use stylex_macros::stylex_panic;

use crate::shared::utils::object::{
  obj_transform_values_then_map_keys, preprocess_object_properties,
};
use stylex_ast::ast::convertors::{convert_key_value_to_str, get_key_values_from_object};
use stylex_constants::constants::messages::VALUES_MUST_BE_OBJECT;
use stylex_state::{evaluate_result_value::EvaluateResultValue, state_manager::StateManager};
use stylex_structures::pair::Pair;
use stylex_types::{
  enums::data_structures::injectable_style::InjectableStyleKind,
  structures::injectable_style::InjectableStyle,
};
use stylex_utils::{hash::create_hash, string::dashify};

/// Animation parts: selector to declarations CSS text.
type ViewTransitionParts = IndexMap<String, String>;

pub(crate) fn stylex_view_transition_class(
  styles: &EvaluateResultValue,
  state: &mut StateManager,
) -> (String, InjectableStyleKind) {
  // `classNamePrefix` is already defaulted to `x`; an empty one is honoured as empty.
  let class_name_prefix = state.options.class_name_prefix.clone();

  let Some(styles) = styles.as_expr().and_then(|expr| expr.as_object()) else {
    stylex_panic!("{}", VALUES_MUST_BE_OBJECT)
  };

  let key_values = get_key_values_from_object(styles);

  let mut parts = ViewTransitionParts::with_capacity(key_values.len());

  for key_value in key_values.iter() {
    let part = convert_key_value_to_str(key_value);

    let entries = preprocess_object_properties(&key_value.value, state);
    let declarations =
      obj_transform_values_then_map_keys(&entries, state, |key| dashify(key).into_owned());

    // Upper bound; empty declarations are dropped below.
    let capacity = declarations
      .iter()
      .map(|pair| pair.key.len() + pair.value.len() + 2)
      .sum();
    let mut css_text = String::with_capacity(capacity);

    for declaration in declarations.iter().filter_map(Pair::as_css_text) {
      css_text.push_str(&declaration);
    }

    parts.insert(format!("::view-transition-{}", dashify(&part)), css_text);
  }

  let view_transition_class_name =
    class_name_prefix + create_hash(&string_to_hash(&parts)).as_str();

  let style = construct_final_view_transition_css_str(&parts, &view_transition_class_name);

  (
    view_transition_class_name,
    InjectableStyleKind::Regular(InjectableStyle {
      ltr: style,
      rtl: None,
      priority: Some(1.0),
    }),
  )
}

fn construct_final_view_transition_css_str(
  parts: &ViewTransitionParts,
  class_name: &str,
) -> String {
  let capacity = parts
    .iter()
    .map(|(part, css_text)| part.len() + class_name.len() + css_text.len() + 6)
    .sum();

  let mut result = String::with_capacity(capacity);

  for (part, css_text) in parts.iter() {
    let _ = write!(result, "{}(*.{}){{{}}}", part, class_name, css_text);
  }

  result
}

/// The `selector:body;` string the class name is hashed from.
/// Empty bodies are kept to match the reference hash; emitted CSS is built elsewhere.
fn string_to_hash(parts: &ViewTransitionParts) -> String {
  let mut result = String::with_capacity(parts.len() * 16);

  for (part, css_text) in parts.iter() {
    let _ = write!(result, "{}:{};", part, css_text);
  }

  result
}
