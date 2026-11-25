use std::rc::Rc;

use stylex_macros::stylex_panic;
use stylex_styleq::{StyleMap, StyleqArgument};

use crate::shared::utils::core::parse_nullable_style::{ResolvedArg, StyleObject};
use stylex_state::{
  flat_compiled_styles_value::FlatCompiledStylesValue, types::FlatCompiledStyles,
};

pub(crate) struct StyleQResult {
  pub(crate) class_name: String,
  pub(crate) inline_style: Option<FlatCompiledStyles>,
  pub(crate) data_style_src: Option<String>,
}

impl StyleqArgument<Rc<FlatCompiledStylesValue>> for ResolvedArg {
  fn as_style(&self) -> Option<&StyleMap<Rc<FlatCompiledStylesValue>>> {
    match self {
      ResolvedArg::StyleObject(style_object) => style_object_as_style(style_object),
      ResolvedArg::ConditionalStyle(_, Some(style_object), _) => {
        style_object_as_style(style_object)
      },
      ResolvedArg::ConditionalStyle(_, None, _) => None,
    }
  }

  /// The style's address; valid only while this merge holds it.
  fn cache_key(&self) -> Option<usize> {
    self
      .as_style()
      .map(|style| std::ptr::from_ref(style).cast::<()>() as usize)
  }

  fn should_skip(&self) -> bool {
    matches!(
      self,
      ResolvedArg::StyleObject(StyleObject::Nullable) | ResolvedArg::ConditionalStyle(_, None, _)
    )
  }
}

pub(crate) fn styleq(arguments: &[ResolvedArg]) -> StyleQResult {
  if arguments.is_empty() {
    return StyleQResult {
      class_name: String::new(),
      inline_style: None,
      data_style_src: None,
    };
  }

  // Cache off: this per-call merger measured zero hits over 1,076 lookups. See the
  // `stylex-styleq` module comment.
  let styleq = stylex_styleq::create_styleq(stylex_styleq::StyleqOptions {
    dedupe_class_name_chunks: true,
    disable_cache: true,
    ..Default::default()
  });
  let result = styleq.styleq(arguments);

  StyleQResult {
    class_name: result.class_name,
    inline_style: result.inline_style,
    data_style_src: Some(result.data_style_src),
  }
}

fn style_object_as_style(style_object: &StyleObject) -> Option<&FlatCompiledStyles> {
  match style_object {
    StyleObject::Style(style) => Some(style),
    StyleObject::Nullable => None,
    StyleObject::Other => {
      stylex_panic!("Only compiled StyleX style objects are allowed in styleq().")
    },
  }
}

#[cfg(test)]
#[path = "tests/styleq_tests.rs"]
mod tests;
