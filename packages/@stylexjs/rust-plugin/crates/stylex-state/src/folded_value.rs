//! What a folded expression holds, in the kind JavaScript gives it.
//!
//! "Folded" and not "evaluated", because
//! [`EvaluateResultValue`](crate::evaluate_result_value::EvaluateResultValue)
//! already carries the second name in this crate and answers a different
//! question: it is what the evaluator hands back, and this is the kind one
//! value of it turned out to be.
//!
//! Two callers read the same thing. A `props` call is given a plain object and
//! keeps the kind of every declaration in it, and a `defineConsts` call keeps
//! the kind of every constant. The reference implementation writes both back
//! through one function, so they are read through one here.
//!
//! It sits beside the value it builds rather than beside either caller, and
//! beside the JSON spelling of that value, which is the same reading in the
//! other direction.

use std::rc::Rc;

use crate::flat_compiled_styles_value::FlatCompiledStylesValue;
use indexmap::IndexMap;
use stylex_ast::ast::convertors::{
  convert_key_value_to_str, convert_str_lit_to_string, convert_tpl_to_string_lit, is_js_undefined,
};
use stylex_macros::{stylex_panic, stylex_unimplemented};
use swc_core::ecma::ast::{Expr, Lit, ObjectLit};

/// The one name an object literal writes that declares no property of its own.
///
/// Writing it sets what the object inherits from, whichever of the three ways
/// it was spelled, so the object holds no such name afterwards.
const PROTOTYPE_KEY: &str = "__proto__";

/// Reads the names one object literal writes into `compiled_styles`, and
/// answers the object it was told to inherit from.
///
/// A name written twice keeps the place of its first writing and the value of
/// its last, which is how the language reads such an object. Only an object
/// can be inherited from, so a prototype the author gave anything else sets
/// nothing and declares nothing.
pub fn read_declarations<'a>(
  compiled_styles: &mut IndexMap<String, Rc<FlatCompiledStylesValue>>,
  ObjectLit { props, .. }: &'a ObjectLit,
) -> Option<&'a ObjectLit> {
  let mut prototype = None;

  for prop in props.iter() {
    // A spread, a method, a getter, a setter and a shorthand name are the
    // spellings this reader cannot name a value for. Refused rather than
    // skipped, which is what the list arm below and every sibling reader do:
    // a skipped declaration is emitted as though the author never wrote it,
    // and a wrong output is worse than a stopped build.
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
///
/// The kind the author wrote is the kind answered here: `0.5` is a number,
/// `true` is a boolean, `undefined` is a value that was never given, and an
/// object or a list holds values read the same way again. Every one of them is
/// written back where the call was, so a kind dropped here is a declaration
/// the runtime never applies or a constant a reader never sees.
pub fn folded_value(expr: &Expr) -> FlatCompiledStylesValue {
  match expr {
    Expr::Lit(lit) => literal_value(lit),
    // A declaration with no value is kept rather than dropped here, because
    // the merge is what decides it declares nothing: it skips the property
    // whole and leaves it for a later style to set.
    Expr::Ident(ident) if is_js_undefined(ident) => FlatCompiledStylesValue::Undefined,
    // A template that spells one piece of text is that text. A folded value
    // reaches here as a string already, so this is for a caller that reads a
    // written expression rather than a folded one.
    Expr::Tpl(tpl) => match convert_tpl_to_string_lit(tpl) {
      Some(lit) => literal_value(&lit),
      None => stylex_unimplemented!("Encountered a style value the compiler cannot read."),
    },
    // A nested object is written back with the names it holds itself. Nothing
    // walks what it inherits from, because nothing merges it -- the runtime is
    // handed the object, and a prototype is not part of what is written.
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
          // A hole and a spread are the two slots that hold no value of their
          // own. Neither survives being read, so the call is left to the
          // runtime rather than written short of an element.
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
    // Read as the text it is rather than through the converter that asks what
    // kind of literal it is: the arm has already answered that.
    Lit::Str(text) => FlatCompiledStylesValue::String(convert_str_lit_to_string(text)),
    Lit::Num(number) => FlatCompiledStylesValue::Number(number.value),
    Lit::Bool(bool_lit) => FlatCompiledStylesValue::Bool(bool_lit.value),
    Lit::Null(_) => FlatCompiledStylesValue::Null,
    _ => {
      stylex_panic!("Encountered a literal a style value cannot hold.");
    },
  }
}
