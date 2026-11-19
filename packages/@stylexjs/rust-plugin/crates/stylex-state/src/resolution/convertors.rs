//! What a declaration spells, read back as a string or an expression.

use stylex_ast::ast::convertors::{convert_lit_to_string, get_expr_from_var_decl};
use stylex_constants::constants::messages::{ILLEGAL_PROP_VALUE, VAR_DECL_INIT_REQUIRED};
use stylex_macros::stylex_panic;
use stylex_structures::raw_value::TRawValue;
use swc_core::ecma::ast::{Expr, Ident, Lit, Tpl};

use crate::{
  functions::FunctionMap, resolution::lookup::get_var_decl_by_ident, state_manager::StateManager,
};

/// The string an expression spells, or `None` when it spells no string — an object, an array, `null`, a boolean.
pub fn convert_expr_to_str(
  expr_string: &Expr,
  state: &mut StateManager,
  functions: &FunctionMap,
) -> Option<String> {
  match &expr_string {
    Expr::Ident(ident) => ident_to_string(ident, state, functions),
    Expr::Lit(lit) => convert_lit_to_string(lit),
    _ => None,
  }
}

/// The string the binding behind an identifier spells, or `None` when it spells no string.
fn ident_to_string(
  ident: &Ident,
  state: &mut StateManager,
  functions: &FunctionMap,
) -> Option<String> {
  let var_decl = get_var_decl_by_ident(ident, state, functions);

  match &var_decl {
    Some(var_decl) => match get_expr_from_var_decl(var_decl) {
      Expr::Lit(lit) => convert_lit_to_string(lit),
      Expr::Ident(ident) => ident_to_string(ident, state, functions),
      _ => None,
    },
    None => None,
  }
}

#[inline]
pub fn convert_ident_to_expr(
  ident: &Ident,
  state: &mut StateManager,
  functions: &FunctionMap,
) -> Expr {
  match get_var_decl_by_ident(ident, state, functions) {
    Some(var_decl) => get_expr_from_var_decl(&var_decl).clone(),
    _ => {
      stylex_panic!("{}", ILLEGAL_PROP_VALUE)
    },
  }
}

/// The template with each substituted identifier replaced by the expression its declaration was initialized with.
pub fn handle_tpl_to_expression(
  tpl: &Tpl,
  state: &mut StateManager,
  functions: &FunctionMap,
) -> Expr {
  let mut tpl = tpl.clone();

  for expr in tpl.exprs.iter_mut() {
    if let Expr::Ident(ident) = expr.as_ref() {
      let var_decl = get_var_decl_by_ident(ident, state, functions);

      if let Some(var_decl) = &var_decl {
        *expr = match var_decl.init.clone() {
          Some(init) => init,
          None => stylex_panic!("{}", VAR_DECL_INIT_REQUIRED),
        };
      }
    };
  }

  Expr::Tpl(tpl)
}

/// Reads a literal as an authored style value, keeping the JS type distinction that decides whether a unit suffix is appended later.
pub fn convert_lit_to_raw_value(lit: &Lit) -> Option<TRawValue> {
  match lit {
    Lit::Num(num) => Some(TRawValue::Number(num.value)),
    _ => convert_lit_to_string(lit).map(TRawValue::String),
  }
}
