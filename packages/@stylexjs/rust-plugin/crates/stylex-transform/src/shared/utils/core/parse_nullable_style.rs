use std::borrow::Cow;
use std::rc::Rc;

use indexmap::IndexMap;
use stylex_ast::ast::convertors::{convert_lit_to_string, is_js_undefined, normalize_expr};
use stylex_macros::stylex_unimplemented;
use swc_core::ecma::ast::{Expr, Lit, MemberProp, ObjectLit};

use stylex_evaluator::evaluate::evaluate_with_functions;

use stylex_state::folded_value::read_declarations;
use stylex_state::{
  evaluate_result_value::EvaluateResultValue,
  flat_compiled_styles_value::FlatCompiledStylesValue,
  functions::FunctionMap,
  state_manager::StateManager,
  types::{FlatCompiledStyles, StylesObjectMap},
};

/// What one `stylex.props`-family argument was read as: a compiled style, an authored null, or
/// something left to the runtime.
#[derive(Debug, PartialEq, Clone)]
pub(crate) enum StyleObject {
  /// A compiled style, shared with the state or owned by this merge.
  Style(Rc<FlatCompiledStyles>),
  Nullable,
  Other,
}

impl StyleObject {
  /// Wraps a state-held or freshly built style.
  #[inline]
  pub(crate) fn style(style: impl Into<Rc<FlatCompiledStyles>>) -> Self {
    StyleObject::Style(style.into())
  }
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum ResolvedArg {
  StyleObject(StyleObject),
  ConditionalStyle(Expr, Option<StyleObject>, Option<StyleObject>),
}

impl ResolvedArg {
  /// Creates a `ResolvedArg::StyleObject`.
  ///
  /// # Example
  /// ```ignore
  /// let arg = ResolvedArg::style_object(StyleObject::style(...));
  /// ```
  #[inline]
  pub(crate) fn style_object(style_obj: StyleObject) -> Self {
    ResolvedArg::StyleObject(style_obj)
  }

  /// Creates a `ResolvedArg::ConditionalStyle`.
  ///
  /// # Example
  /// ```ignore
  /// let arg = ResolvedArg::conditional(test, Some(primary), Some(fallback));
  /// ```
  #[inline]
  pub(crate) fn conditional(
    test: Expr,
    primary: Option<StyleObject>,
    fallback: Option<StyleObject>,
  ) -> Self {
    ResolvedArg::ConditionalStyle(test, primary, fallback)
  }
}

pub(crate) fn parse_nullable_style(
  path: &Expr,
  state: &mut StateManager,
  evaluate_path_fn_config: &Rc<FunctionMap>,
) -> StyleObject {
  // Read through parentheses: `stylex.props((styles.root))`.
  let path = normalize_expr(path);

  // Calls (`_temp.color(c)`, `styles.opacity(1)`) always bail out to the runtime.
  if path.is_call() {
    return StyleObject::Other;
  }

  let result = match path {
    Expr::Lit(lit) => {
      if let Lit::Null(_) = lit {
        StyleObject::Nullable
      } else {
        StyleObject::Other
      }
    },
    Expr::Ident(ident) => {
      if is_js_undefined(ident) {
        StyleObject::Nullable
      } else {
        StyleObject::Other
      }
    },
    Expr::Member(member) => {
      let mut namespaces: Option<&StylesObjectMap> = None;
      let mut obj_name: Option<&str> = None;
      let mut prop_name: Option<Cow<'_, str>> = None;

      if let Some(obj_ident) = normalize_expr(&member.obj).as_ident()
        && let Some(style_var_namespaces) = state.style_var_namespaces(obj_ident)
      {
        match &member.prop {
          MemberProp::Ident(prop_ident) => {
            namespaces = Some(style_var_namespaces);
            obj_name = Some(obj_ident.sym.as_str());
            prop_name = Some(Cow::Borrowed(prop_ident.sym.as_str()));
          },
          MemberProp::Computed(computed) => {
            if let Some(lit) = normalize_expr(&computed.expr).as_lit() {
              namespaces = Some(style_var_namespaces);
              obj_name = Some(obj_ident.sym.as_str());
              prop_name = convert_lit_to_string(lit).map(Cow::Owned);
            }
          },
          MemberProp::PrivateName(_) => {},
        }
      }

      if let Some(namespaces) = namespaces
        && let Some(obj_name) = obj_name
        && let Some(prop_name) = prop_name
      {
        // Dynamic style functions bail out to the runtime.
        if state
          .dynamic_style_namespaces
          .get(obj_name)
          .is_some_and(|namespaces| namespaces.contains(prop_name.as_ref()))
        {
          return StyleObject::Other;
        }

        if let Some(style_value) = namespaces.get(prop_name.as_ref()) {
          return StyleObject::style(Rc::clone(style_value));
        }
        // A known generated style object with an unknown namespace stays a runtime call.
        return StyleObject::Other;
      }

      StyleObject::Other
    },
    _ => StyleObject::Other,
  };

  if result == StyleObject::Other {
    let parsed_obj = evaluate_with_functions(path, state, Rc::clone(evaluate_path_fn_config));

    if parsed_obj.confident
      && let Some(result) = parsed_obj.value.as_ref()
    {
      let mut compiled_styles: IndexMap<String, Rc<FlatCompiledStylesValue>> = IndexMap::new();

      if let Some(value) = parse_compiled_styles(&mut compiled_styles, result, state) {
        return value;
      }
    }
  }

  result
}

fn parse_compiled_styles(
  compiled_styles: &mut IndexMap<String, Rc<FlatCompiledStylesValue>>,
  result: &EvaluateResultValue,
  state: &StateManager,
) -> Option<StyleObject> {
  if matches!(result, EvaluateResultValue::ThemeRef(_)) {
    return Some(StyleObject::Other);
  }
  let value = match stylex_evaluator::evaluate::materialize_result(result, state) {
    Ok(Some(value)) => value,
    Ok(None)
      if matches!(
        result,
        EvaluateResultValue::Vec(_)
          | EvaluateResultValue::SharedArray(_)
          | EvaluateResultValue::NativeArray(_)
      ) =>
    {
      stylex_unimplemented!("Encountered an element of a style list the compiler cannot read.")
    },
    Ok(None) => stylex_unimplemented!("Encountered a style argument the compiler cannot read."),
    Err(error) => stylex_macros::stylex_panic!("{}", error.message),
  };
  match normalize_expr(&value) {
    Expr::Object(_) => {
      parse_nullable_object(compiled_styles, &value);
      Some(StyleObject::style(std::mem::take(compiled_styles)))
    },
    Expr::Array(array) => {
      read_array(compiled_styles, array);
      if compiled_styles.is_empty() {
        Some(StyleObject::Other)
      } else {
        Some(StyleObject::style(std::mem::take(compiled_styles)))
      }
    },
    _ => None,
  }
}
/// Flattens materialized data once in authored order; null slots stay nullable.
fn read_array(
  compiled_styles: &mut IndexMap<String, Rc<FlatCompiledStylesValue>>,
  array: &swc_core::ecma::ast::ArrayLit,
) {
  for element in &array.elems {
    let Some(element) = element else {
      continue;
    };
    match normalize_expr(&element.expr) {
      Expr::Object(_) => parse_nullable_object(compiled_styles, &element.expr),
      Expr::Array(array) => read_array(compiled_styles, array),
      Expr::Lit(Lit::Null(_)) => {},
      Expr::Ident(id) if is_js_undefined(id) => {},
      _ => {
        stylex_unimplemented!("Encountered an element of a style list the compiler cannot read.")
      },
    }
  }
}

fn parse_nullable_object(
  compiled_styles: &mut IndexMap<String, Rc<FlatCompiledStylesValue>>,
  expr: &Expr,
) {
  match expr {
    Expr::Object(object) => {
      for (key, value) in declarations_of(object) {
        compiled_styles.insert(key, value);
      }
    },
    _ => {
      stylex_unimplemented!("Encountered a style argument that is not an object.");
    },
  }
}

/// Every declaration one style argument makes: own names (last write wins), then inherited
/// prototype names it doesn't shadow.
fn declarations_of(object: &ObjectLit) -> IndexMap<String, Rc<FlatCompiledStylesValue>> {
  let mut declarations: IndexMap<String, Rc<FlatCompiledStylesValue>> =
    IndexMap::with_capacity(object.props.len());
  let mut prototype = read_declarations(&mut declarations, object);

  while let Some(object) = prototype {
    let mut inherited: IndexMap<String, Rc<FlatCompiledStylesValue>> =
      IndexMap::with_capacity(object.props.len());

    prototype = read_declarations(&mut inherited, object);

    for (key, value) in inherited {
      declarations.entry(key).or_insert(value);
    }
  }

  declarations
}

#[cfg(test)]
#[path = "tests/parse_nullable_style_tests.rs"]
mod tests;
