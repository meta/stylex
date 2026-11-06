//! Style arguments the merge cases below are built from.
//!
//! `styleq`, `props` and `attrs` are one chain read by three sets of names, so
//! they are given their inputs by one set of builders.

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
///
/// A trait rather than two free functions, so a case reads a result the way it
/// reads every other value. Here rather than beside the type, because the arm
/// that answers for the kind a result does not hold is a question only a case
/// asks.
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

/// A compiled namespace: the marker every compiled style carries, and the
/// properties given as `(property, class name)`.
pub(crate) fn compiled(properties: &[(&str, &str)]) -> FlatCompiledStyles {
  let mut styles = inline(properties);

  styles.shift_insert(
    0,
    COMPILED_KEY.to_owned(),
    Rc::new(FlatCompiledStylesValue::Bool(true)),
  );

  styles
}

/// A style the compiler did not compile: the same properties, carrying no
/// marker, so the merge writes it out as an inline style rather than as
/// classes.
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

/// An inline style holding one declaration of the given kind.
///
/// The builder above spells text alone, and an inline style holds every kind
/// the author can write.
pub(crate) fn inline_value(property: &str, value: FlatCompiledStylesValue) -> FlatCompiledStyles {
  let mut styles: FlatCompiledStyles = IndexMap::new();

  styles.insert(property.to_owned(), Rc::new(value));

  styles
}

/// An inline style holding a value kind no inline style can hold.
///
/// An injectable style is written where a rule is collected, never where a
/// declaration is, so it stands for the kinds the two writers of an inline
/// style have no spelling for. It is built here because no source produces
/// one in this place.
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

/// One style argument holding the given namespace.
pub(crate) fn style_of(properties: &[(&str, &str)]) -> ResolvedArg {
  ResolvedArg::style_object(StyleObject::style(compiled(properties)))
}

/// One style argument holding the given namespace as it is.
pub(crate) fn styles(styles: FlatCompiledStyles) -> ResolvedArg {
  ResolvedArg::style_object(StyleObject::style(styles))
}
