use std::borrow::Cow;
use std::rc::Rc;

use indexmap::IndexMap;
use stylex_css::css::common::inline_style_to_css_string;
use stylex_structures::pair::PairCow;

use crate::shared::enums::data_structures::fn_result::FnResult;
use stylex_state::flat_compiled_styles_value::FlatCompiledStylesValue;

use super::{parse_nullable_style::ResolvedArg, props::props_map};

/// The HTML attributes a `stylex.attrs(...)` call is replaced by.
/// Order is `class`, `style`, `data-style-src`, matching the runtime; it is part of the output.
pub(crate) fn attrs(styles: &[ResolvedArg]) -> FnResult {
  let attrs = props_map(styles);

  let mut attrs_map: IndexMap<String, Rc<FlatCompiledStylesValue>> = IndexMap::new();

  if let Some(class_name) = attrs.get("className") {
    attrs_map.insert("class".to_string(), class_name.clone());
  }

  // Serialize the inline style object as `style` attribute CSS text.
  if let Some(FlatCompiledStylesValue::Object(style)) = attrs.get("style").map(Rc::as_ref) {
    let declarations = style
      .iter()
      .map(|(key, value)| PairCow {
        key: Cow::Borrowed(key.as_str()),
        value: value.to_js_text(),
      })
      .collect::<Vec<PairCow<'_>>>();

    attrs_map.insert(
      "style".to_string(),
      Rc::new(FlatCompiledStylesValue::String(inline_style_to_css_string(
        &declarations,
      ))),
    );
  }

  if let Some(data_style_src) = attrs.get("data-style-src") {
    attrs_map.insert("data-style-src".to_string(), data_style_src.clone());
  }

  FnResult::Values(attrs_map)
}
