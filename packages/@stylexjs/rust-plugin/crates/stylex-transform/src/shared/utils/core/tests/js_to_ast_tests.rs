//! The object expression the compiled styles are written back as.

use std::rc::Rc;

use indexmap::IndexMap;
use stylex_state::{
  flat_compiled_styles_value::FlatCompiledStylesValue, types::FlatCompiledStyles,
};
use stylex_types::structures::injectable_style::InjectableStyle;

use crate::shared::utils::core::js_to_ast::convert_values_to_ast;

fn values_of(pairs: &[(&str, FlatCompiledStylesValue)]) -> FlatCompiledStyles {
  let mut values: FlatCompiledStyles = IndexMap::new();

  for (key, value) in pairs {
    values.insert((*key).to_owned(), Rc::new(value.clone()));
  }

  values
}

/// The inline-style writer reads by its own rules.
#[test]
#[should_panic(expected = "Encountered an unsupported value type during AST conversion.")]
fn refuses_an_inline_value_it_cannot_write() {
  convert_values_to_ast(&values_of(&[(
    "style",
    FlatCompiledStylesValue::Object(inline_style(&[("color", unwritable())])),
  )]));
}

/// An injectable style is collected where a rule is, never where a declaration is.
fn unwritable() -> FlatCompiledStylesValue {
  FlatCompiledStylesValue::InjectableStyle(InjectableStyle {
    ltr: ".x1e2nbdu{color:red}".to_owned(),
    rtl: None,
    priority: Some(3000.0),
  })
}

#[test]
#[should_panic(expected = "A style value that is undefined cannot be written.")]
fn refuses_a_list_holding_a_value_that_is_undefined() {
  convert_values_to_ast(&values_of(&[(
    "style",
    FlatCompiledStylesValue::Object(inline_style(&[(
      "color",
      FlatCompiledStylesValue::List(vec![Rc::new(FlatCompiledStylesValue::Undefined)]),
    )])),
  )]));
}

fn inline_style(declarations: &[(&str, FlatCompiledStylesValue)]) -> FlatCompiledStyles {
  values_of(declarations)
}
