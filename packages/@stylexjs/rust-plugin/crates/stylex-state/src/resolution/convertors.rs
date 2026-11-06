//! What a declaration spells, read back as a string or an expression.
//!
//! Every reader here resolves through [`super::lookup`] and stops at the first
//! thing that is not a literal or another identifier. None of them folds: a
//! conversion that needs a binary expression evaluated lives above this crate,
//! and that split is what keeps this half free of the evaluation cycle.

use stylex_ast::ast::convertors::{convert_lit_to_string, get_expr_from_var_decl};
use stylex_constants::constants::messages::{ILLEGAL_PROP_VALUE, VAR_DECL_INIT_REQUIRED};
use stylex_macros::stylex_panic;
use stylex_structures::raw_value::TRawValue;
use swc_core::ecma::ast::{Expr, Ident, Lit, Tpl};

use crate::{
  functions::FunctionMap, resolution::lookup::get_var_decl_by_ident, state_manager::StateManager,
};

/// The string an expression spells, or `None` when it spells no string —
/// an object, an array, `null`, a boolean.
///
/// `None` is the answer for every non-string rather than a panic, because what
/// a non-string means is the caller's question, not the converter's: a step of
/// an animation declares nothing, a namespace name is a hard error. Answering
/// it here would force one of those onto the other.
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

/// The string the binding behind an identifier spells, or `None` when it spells
/// no string.
///
/// `None` covers both an identifier bound to something that is not a string and
/// one with no binding to read at all -- `undefined`, which is an ordinary
/// global identifier rather than a literal. Neither is decided here, for the
/// reason given on [`convert_expr_to_str`].
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

/// The template with each substituted identifier replaced by the expression its
/// declaration was initialized with.
///
/// An identifier no declaration binds is left in place rather than refused: the
/// template still has to be read as a whole, and what an unresolved
/// substitution means belongs to the caller.
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
        // A declaration with no initializer spells no expression to substitute.
        *expr = match var_decl.init.clone() {
          Some(init) => init,
          None => stylex_panic!("{}", VAR_DECL_INIT_REQUIRED),
        };
      }
    };
  }

  Expr::Tpl(tpl)
}

/// Reads a literal as an authored style value, keeping the JS type distinction
/// that decides whether a unit suffix is appended later.
///
/// A numeric literal stays a number; everything else that has a string form
/// becomes one. `None` means the literal is not a usable style value.
pub fn convert_lit_to_raw_value(lit: &Lit) -> Option<TRawValue> {
  match lit {
    Lit::Num(num) => Some(TRawValue::Number(num.value)),
    _ => convert_lit_to_string(lit).map(TRawValue::String),
  }
}
