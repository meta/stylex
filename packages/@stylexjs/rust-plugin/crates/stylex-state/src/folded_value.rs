use std::rc::Rc;

use crate::flat_compiled_styles_value::FlatCompiledStylesValue;
use indexmap::IndexMap;
use stylex_ast::ast::convertors::{
  convert_key_value_to_str, convert_str_lit_to_string, convert_tpl_to_string_lit, is_js_undefined,
};
use stylex_macros::{stylex_panic, stylex_unimplemented};
use swc_core::ecma::ast::{Expr, Lit, ObjectLit};

const PROTOTYPE_KEY: &str = "__proto__";

/// Reads the names one object literal writes into `compiled_styles`, and answers the object it was told to inherit from.
pub fn read_declarations<'a>(
  compiled_styles: &mut IndexMap<String, Rc<FlatCompiledStylesValue>>,
  ObjectLit { props, .. }: &'a ObjectLit,
) -> Option<&'a ObjectLit> {
  let mut prototype = None;

  for prop in props.iter() {
    let Some(key_value) = prop.as_prop().and_then(|p| p.as_key_value()) else {
      stylex_unimplemented!("Encountered a declaration the compiler cannot read.");
    };

    let key = convert_key_value_to_str(key_value);

    if key == PROTOTYPE_KEY {
      prototype = key_value.value.as_object();
      continue;
    }

    compiled_styles.insert(key, Rc::new(folded_value(key_value.value.as_ref())));
  }

  prototype
}

/// What one folded expression holds.
/// Every one of them is written back where the call was, so a kind dropped here is a declaration the runtime never applies or a constant a reader never sees.
pub fn folded_value(expr: &Expr) -> FlatCompiledStylesValue {
  match expr {
    Expr::Lit(lit) => literal_value(lit),

    Expr::Ident(ident) if is_js_undefined(ident) => FlatCompiledStylesValue::Undefined,

    Expr::Tpl(tpl) => match convert_tpl_to_string_lit(tpl) {
      Some(lit) => literal_value(&lit),
      None => stylex_unimplemented!("Encountered a style value the compiler cannot read."),
    },

    Expr::Object(object) => {
      let mut nested: IndexMap<String, Rc<FlatCompiledStylesValue>> =
        IndexMap::with_capacity(object.props.len());

      read_declarations(&mut nested, object);

      FlatCompiledStylesValue::Object(nested)
    },
    Expr::Array(array) => FlatCompiledStylesValue::List(
      array
        .elems
        .iter()
        .map(|element| match element {
          Some(element) if element.spread.is_none() => Rc::new(folded_value(&element.expr)),

          _ => stylex_unimplemented!("Encountered a list element with no value of its own."),
        })
        .collect(),
    ),
    _ => {
      stylex_unimplemented!("Encountered a style value the compiler cannot read.");
    },
  }
}

fn literal_value(lit: &Lit) -> FlatCompiledStylesValue {
  match lit {
    Lit::Str(text) => FlatCompiledStylesValue::String(convert_str_lit_to_string(text)),
    Lit::Num(number) => FlatCompiledStylesValue::Number(number.value),
    Lit::Bool(bool_lit) => FlatCompiledStylesValue::Bool(bool_lit.value),
    Lit::Null(_) => FlatCompiledStylesValue::Null,
    _ => {
      stylex_panic!("Encountered a literal a style value cannot hold.");
    },
  }
}
