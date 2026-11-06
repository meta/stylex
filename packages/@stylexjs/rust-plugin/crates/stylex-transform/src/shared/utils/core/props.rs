use std::rc::Rc;

use indexmap::IndexMap;

use crate::shared::{
  enums::data_structures::fn_result::FnResult,
  utils::core::styleq::{StyleQResult, styleq},
};
use stylex_state::{
  flat_compiled_styles_value::FlatCompiledStylesValue, types::FlatCompiledStyles,
};

use super::parse_nullable_style::ResolvedArg;

/// The properties a `stylex.props(...)` call is replaced by.
pub(crate) fn props(styles: &[ResolvedArg]) -> FnResult {
  FnResult::Values(props_map(styles))
}

/// The properties the merged styles become, before they are named as a result.
///
/// Split out because `attrs` needs the same map and reads it by another set of
/// names. Asking `props` for it would have `attrs` take the result apart again,
/// and every step of that is a case the result can never be in.
pub(crate) fn props_map(styles: &[ResolvedArg]) -> FlatCompiledStyles {
  let StyleQResult {
    class_name,
    inline_style,
    data_style_src,
  } = styleq(styles);

  // Left unsized on purpose. Three names at most are written, and the first
  // insert already reserves three, so a reservation here saves no growth and
  // costs an allocation when the merge writes nothing.
  let mut props_map: FlatCompiledStyles = IndexMap::new();

  if !class_name.is_empty() {
    props_map.insert(
      "className".to_string(),
      Rc::new(FlatCompiledStylesValue::String(class_name)),
    );
  }

  // An empty merge writes no `style` at all, which is what the runtime writes:
  // it asks whether the merged object holds a name before it sets the property.
  // No merge reaches this reading today -- the merger builds an inline object
  // only for a property that has a value -- so this states the rule rather than
  // repairing an output.
  if let Some(inline_style) = inline_style.filter(|style| !style.is_empty()) {
    // The merged declarations are carried on as they are. Each name is kept as
    // the author spelled it, because the runtime reads this property as a style
    // object and `marginTop` is the name such an object carries; the CSS
    // spelling is asked for where CSS text is made. Each value keeps its kind
    // for the same reason: `opacity: 0.5` is a number where the author wrote
    // it, and a `style` property holding text there would be a different
    // declaration.
    props_map.insert(
      "style".to_string(),
      Rc::new(FlatCompiledStylesValue::Object(inline_style)),
    );
  }

  if let Some(data_style_src) = data_style_src
    && !data_style_src.is_empty()
  {
    props_map.insert(
      "data-style-src".to_string(),
      Rc::new(FlatCompiledStylesValue::String(data_style_src)),
    );
  }

  props_map
}

#[cfg(test)]
#[path = "tests/props_tests.rs"]
mod tests;
