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

  /// The address of the style, which names it only while the merge holds it.
  ///
  /// Safe because the merger is built and dropped inside one merge, below, so
  /// every style outlives the cache that names it. A merger that lived longer
  /// would have to refuse this key for a style the reader built, which is
  /// dropped with the merge, and give it only for a style read from the state.
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

  // The cache is off, because this merger cannot hit it. It is built here and
  // dropped when the call returns, so a hit needs the same style twice at the
  // same position of one merge. Measured over the whole transform suite and the
  // fixture corpus: 1,076 lookups, none of them a hit. Every one of those paid
  // for an entry, three shared strings, a chunk and a child map, and collected
  // nothing. The cache is transparent -- a merge answers the same with it and
  // without it -- so turning it off changes what the merge costs and not what
  // it says.
  //
  // A merger that lives for the file does answer lookups from the cache, and
  // saves less than one percent for it. The module comment of `stylex-styleq`
  // holds both measurements.
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
