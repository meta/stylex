// Import error handling macros from shared utilities
use stylex_macros::{
  as_expr_or_panic, stylex_bail, stylex_panic, stylex_unimplemented, unwrap_or_panic,
};
use swc_core::ecma::ast::{BinExpr, Expr, Ident, Lit, Tpl, UnaryExpr, UnaryOp};

use crate::evaluate::{binary_expr_to_num_or_str, evaluate_cached};
use crate::state::EvaluationState;
use stylex_ast::ast::convertors::{
  convert_lit_to_number, convert_lit_to_string, get_expr_from_var_decl,
};
use stylex_constants::constants::messages::{ILLEGAL_PROP_VALUE, non_static_value};
use stylex_js::{coercions, operators::evaluate_bin_expr};
use stylex_state::resolution::lookup::get_var_decl_by_ident;
use stylex_state::{functions::FunctionMap, state_manager::StateManager};
use stylex_utils::swc::get_expr_node_kind;

pub fn expr_to_num(
  expr_num: &Expr,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Result<f64, anyhow::Error> {
  let result = match &expr_num {
    // `undefined`, `NaN` and `Infinity` are ordinary identifiers rather than
    // literals, and the evaluator now hands two of them back as values in their
    // own right — `void x`, and a key an object does not carry. Asked of the
    // binding table instead, they resolve to no declaration and fail the build,
    // where JavaScript answers `NaN`, `NaN` and `Infinity`. The shared coercion
    // names exactly those three and refuses every other identifier, so an
    // ordinary binding still reads its declaration below.
    Expr::Ident(ident) => match coercions::to_js_number(expr_num) {
      Some(number) => number,
      None => ident_to_number(ident, state, traversal_state, &FunctionMap::default()),
    },
    // A string's number is `StringToNumber`, not Rust's float grammar: Rust
    // accepts `inf` and `nan`, which the language does not, and rejects the
    // radix prefixes and the surrounding whitespace, which it does. The first
    // disagreement puts a wrong number in a stylesheet and the second refuses a
    // fold the reference implementation makes. `null` is nought for the same
    // reason: the literal grammar has no reading of it and the language does.
    Expr::Lit(Lit::Str(_) | Lit::Null(_)) => match coercions::to_js_number(expr_num) {
      Some(number) => number,
      None => stylex_bail!("{}", ILLEGAL_PROP_VALUE),
    },
    Expr::Lit(lit) => return convert_lit_to_number(lit),
    Expr::Unary(unary) => convert_unary_to_num(unary, state, traversal_state, fns),
    Expr::Bin(lit) => {
      let mut state = Box::new(EvaluationState::new());

      match binary_expr_to_num_or_str(lit, &mut state, traversal_state, fns)?.as_number() {
        Some(number) => number,
        None => stylex_bail!(
          "Binary expression is not a number: {}",
          get_expr_node_kind(expr_num)
        ),
      }
    },
    // An expression with no numeric reading is reported rather than fatal:
    // `-{}` and `Math.abs([])` are ordinary JavaScript, and this returns a
    // `Result` precisely so the evaluator can refuse to fold them instead of
    // aborting the build from inside an evaluation allowed to fail.
    _ => stylex_bail!(
      "Expression is not a number: {}",
      get_expr_node_kind(expr_num)
    ),
  };

  Result::Ok(result)
}

pub fn convert_unary_to_num(
  unary_expr: &UnaryExpr,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> f64 {
  let arg = unary_expr.arg.as_ref();
  let op = unary_expr.op;

  match &op {
    UnaryOp::Minus => match expr_to_num(arg, state, traversal_state, fns) {
      Ok(result) => -result,
      Err(error) => stylex_panic!("{}", error),
    },
    UnaryOp::Plus => match expr_to_num(arg, state, traversal_state, fns) {
      Ok(result) => result,
      Err(error) => stylex_panic!("{}", error),
    },
    _ => stylex_panic!(
      "Union operation '{}' is invalid",
      get_expr_node_kind(&Expr::from(unary_expr.clone()))
    ),
  }
}

pub fn ident_to_number(
  ident: &Ident,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> f64 {
  let var_decl = get_var_decl_by_ident(ident, traversal_state, fns);

  match &var_decl {
    Some(var_decl) => {
      let var_decl_expr = get_expr_from_var_decl(var_decl);

      match &var_decl_expr {
        Expr::Bin(bin_expr) => {
          match binary_expr_to_num_or_str(bin_expr, state, traversal_state, fns)
            .unwrap_or_else(|error| stylex_panic!("{}", error))
            .as_number()
          {
            Some(number) => number,
            None => stylex_panic!(
              "Binary expression is not a number: {}",
              get_expr_node_kind(var_decl_expr)
            ),
          }
        },
        Expr::Unary(unary_expr) => convert_unary_to_num(unary_expr, state, traversal_state, fns),
        Expr::Lit(lit) => {
          convert_lit_to_number(lit).unwrap_or_else(|error| stylex_panic!("{}", error))
        },
        _ => stylex_panic!(
          "Variable {} is not a number",
          get_expr_node_kind(var_decl_expr)
        ),
      }
    },
    None => {
      stylex_panic!("Variable {} is not declared", ident.sym)
    },
  }
}

pub fn expr_tpl_to_string(
  tpl: &Tpl,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> String {
  let quasi_len = tpl.quasis.iter().map(|quasi| quasi.raw.len()).sum();
  let mut tpl_str = String::with_capacity(quasi_len);

  for (i, quasi) in tpl.quasis.iter().enumerate() {
    tpl_str.push_str(quasi.raw.as_ref());

    if i < tpl.exprs.len() {
      match &tpl.exprs[i].as_ref() {
        Expr::Ident(ident) => {
          let ident = get_var_decl_by_ident(ident, traversal_state, fns);

          match ident {
            Some(var_decl) => {
              let var_decl_expr = get_expr_from_var_decl(&var_decl);

              let value = match &var_decl_expr {
                Expr::Lit(lit) => match convert_lit_to_string(lit) {
                  Some(s) => s,
                  None => stylex_panic!("{}", ILLEGAL_PROP_VALUE),
                },
                _ => stylex_panic!("{}", ILLEGAL_PROP_VALUE),
              };

              tpl_str.push_str(value.as_str());
            },
            None => stylex_panic!("{}", non_static_value("expr_tpl_to_string")),
          }
        },
        Expr::Bin(bin) => tpl_str.push_str(
          transform_bin_expr_to_number(bin, state, traversal_state, fns)
            .to_string()
            .as_str(),
        ),
        Expr::Lit(lit) => tpl_str.push_str(&match convert_lit_to_string(lit) {
          Some(s) => s,
          None => stylex_panic!("{}", ILLEGAL_PROP_VALUE),
        }),
        _ => stylex_unimplemented!("TPL expression: {}", get_expr_node_kind(&tpl.exprs[i])),
      }
    }
  }

  tpl_str
}

pub fn transform_bin_expr_to_number(
  bin: &BinExpr,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> f64 {
  let op = bin.op;
  let Some(left) = evaluate_cached(&bin.left, state, traversal_state, fns) else {
    {
      stylex_panic!(
        "Left expression is not a number: {}",
        get_expr_node_kind(&bin.left)
      )
    }
  };

  let Some(right) = evaluate_cached(&bin.right, state, traversal_state, fns) else {
    {
      stylex_panic!(
        "Right expression is not a number: {}",
        get_expr_node_kind(&bin.right)
      )
    }
  };

  let left_expr = as_expr_or_panic!(left, "Left argument not expression");
  let right_expr = as_expr_or_panic!(right, "Right argument not expression");

  let left = unwrap_or_panic!(expr_to_num(left_expr, state, traversal_state, fns));
  let right = unwrap_or_panic!(expr_to_num(right_expr, state, traversal_state, fns));

  evaluate_bin_expr(op, left, right)
}
