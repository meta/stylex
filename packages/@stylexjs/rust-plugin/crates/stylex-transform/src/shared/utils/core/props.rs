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

/// The merged properties map, shared by `props` and `attrs`.
pub(crate) fn props_map(styles: &[ResolvedArg]) -> FlatCompiledStyles {
  let StyleQResult {
    class_name,
    inline_style,
    data_style_src,
  } = styleq(styles);

  let mut props_map: FlatCompiledStyles = IndexMap::new();

  if !class_name.is_empty() {
    props_map.insert(
      "className".to_string(),
      Rc::new(FlatCompiledStylesValue::String(class_name)),
    );
  }

  // An empty merge writes no `style`, as the runtime does.
  if let Some(inline_style) = inline_style.filter(|style| !style.is_empty()) {
    // Keep authored names (`marginTop`) and value kinds: the runtime reads a style object.
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
