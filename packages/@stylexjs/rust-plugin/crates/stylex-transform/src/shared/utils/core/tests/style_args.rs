//! Style-argument builders shared by the `styleq`, `props` and `attrs` cases.

use std::rc::Rc;

use indexmap::IndexMap;
use stylex_constants::constants::common::COMPILED_KEY;
use stylex_state::{
  flat_compiled_styles_value::FlatCompiledStylesValue, types::FlatCompiledStyles,
};
use stylex_types::structures::injectable_style::InjectableStyle;
use swc_core::ecma::ast::Expr;

use crate::shared::enums::data_structures::fn_result::FnResult;
use crate::shared::utils::core::parse_nullable_style::{ResolvedArg, StyleObject};

/// Reads a result the way a case wants to name it.
pub(crate) trait ResultReader {
  fn as_class_name(&self) -> Option<&Expr>;
  fn as_values(&self) -> Option<&FlatCompiledStyles>;
}

impl ResultReader for FnResult {
  fn as_class_name(&self) -> Option<&Expr> {
    match self {
      FnResult::ClassName(class_name) => Some(class_name),
      FnResult::Values(_) => None,
    }
  }

  fn as_values(&self) -> Option<&FlatCompiledStyles> {
    match self {
      FnResult::Values(values) => Some(values),
      FnResult::ClassName(_) => None,
    }
  }
}

/// A compiled namespace: the marker plus `(property, class name)` pairs.
pub(crate) fn compiled(properties: &[(&str, &str)]) -> FlatCompiledStyles {
  let mut styles = inline(properties);

  styles.shift_insert(
    0,
    COMPILED_KEY.to_owned(),
    Rc::new(FlatCompiledStylesValue::Bool(true)),
  );

  styles
}

/// An uncompiled style (no marker), which the merge writes out inline.
pub(crate) fn inline(properties: &[(&str, &str)]) -> FlatCompiledStyles {
  let mut styles: FlatCompiledStyles = IndexMap::new();

  for (property, value) in properties {
    styles.insert(
      (*property).to_owned(),
      Rc::new(FlatCompiledStylesValue::String((*value).to_owned())),
    );
  }

  styles
}

pub(crate) fn inline_value(property: &str, value: FlatCompiledStylesValue) -> FlatCompiledStyles {
  let mut styles: FlatCompiledStyles = IndexMap::new();

  styles.insert(property.to_owned(), Rc::new(value));

  styles
}

/// An inline style holding an injectable style, which neither inline writer can spell. No
/// source produces one here.
pub(crate) fn inline_unwritable(property: &str) -> FlatCompiledStyles {
  inline_value(
    property,
    FlatCompiledStylesValue::InjectableStyle(InjectableStyle {
      ltr: ".x1e2nbdu{color:red}".to_owned(),
      rtl: None,
      priority: Some(3000.0),
    }),
  )
}

pub(crate) fn style_of(properties: &[(&str, &str)]) -> ResolvedArg {
  ResolvedArg::style_object(StyleObject::style(compiled(properties)))
}

pub(crate) fn styles(styles: FlatCompiledStyles) -> ResolvedArg {
  ResolvedArg::style_object(StyleObject::style(styles))
}
