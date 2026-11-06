use stylex_ast::ast::convertors::create_number_expr;
use swc_core::{
  common::DUMMY_SP,
  ecma::{
    ast::{BinaryOp, Expr, KeyValueProp, Prop, PropName, PropOrSpread, UnaryExpr, UnaryOp},
    utils::quote_ident,
  },
};

use crate::shared::enums::data_structures::fn_result::FnResult;
use stylex_ast::ast::factories::{
  create_bin_expr, create_computed_member_prop, create_member_expr, create_object_expression,
};

use super::{js_to_ast::convert_values_to_ast, parse_nullable_style::ResolvedArg};

fn fn_result_to_expression(fn_result: FnResult) -> Expr {
  match fn_result {
    FnResult::ClassName(class_name) => class_name,
    FnResult::Values(values) => convert_values_to_ast(&values),
  }
}

pub(crate) fn make_string_expression(
  values: &[ResolvedArg],
  props_like_fn: fn(&[ResolvedArg]) -> FnResult,
) -> Expr {
  let conditions = values
    .iter()
    .filter_map(|value| match value {
      ResolvedArg::ConditionalStyle(expr, _, _) => Some(expr),
      _ => None,
    })
    .collect::<Vec<_>>();

  // Split where the emptiness is decided, so the join below is handed a first
  // condition rather than indexing for one. What the set holds is then part of
  // what the join is asked, instead of an invariant argued from here.
  let Some((first_condition, rest_conditions)) = conditions.split_first() else {
    return fn_result_to_expression(props_like_fn(values));
  };

  let condition_permutations = gen_condition_permutations(conditions.len());

  let obj_entries = condition_permutations
    .iter()
    .map(|permutation| {
      let mut i = 0;

      let args = values
        .iter()
        .filter_map(|arg| match arg {
          ResolvedArg::StyleObject(_) => Some(arg.clone()),
          ResolvedArg::ConditionalStyle(_test, primary, fallback) => {
            let result = if permutation.get(i).unwrap_or(&false) == &true {
              primary
            } else {
              fallback
            };

            i += 1;

            result
              .as_ref()
              .map(|result| ResolvedArg::StyleObject(result.clone()))
          },
        })
        .collect::<Vec<ResolvedArg>>();

      let key = permutation
        .iter()
        .fold(0, |so_far, &b| (so_far << 1) | if b { 1 } else { 0 });

      PropOrSpread::Prop(Box::new(Prop::from(KeyValueProp {
        key: PropName::Ident(quote_ident!(key.to_string())),
        value: Box::new(fn_result_to_expression(props_like_fn(&args))),
      })))
    })
    .collect::<Vec<PropOrSpread>>();

  let obj_expressions = create_object_expression(obj_entries);
  let conditions_to_key = gen_bitwise_or_of_conditions(first_condition, rest_conditions);

  Expr::from(create_member_expr(
    obj_expressions,
    create_computed_member_prop(*conditions_to_key),
  ))
}

/// The key an author's conditions read at runtime: each condition shifted to
/// its own bit, the bits joined.
///
/// The first condition is asked for on its own, so a caller cannot ask for the
/// key of no conditions at all -- the set the join starts from is one the
/// signature guarantees.
///
/// The conditions are borrowed from the arguments they were read out of. Each
/// is copied once, into the expression built from it. A set copied to be passed
/// here copied every condition twice.
fn gen_bitwise_or_of_conditions(first: &Expr, rest: &[&Expr]) -> Box<Expr> {
  let count = rest.len() + 1;

  // `!!condition << shift`: the double negation makes the author's value a
  // boolean, and the shift gives each condition a bit of its own.
  let shifted = |index: usize, condition: &Expr| {
    create_bin_expr(
      BinaryOp::LShift,
      Expr::from(UnaryExpr {
        span: DUMMY_SP,
        op: UnaryOp::Bang,
        arg: Box::new(Expr::from(UnaryExpr {
          span: DUMMY_SP,
          op: UnaryOp::Bang,
          arg: Box::new(condition.clone()),
        })),
      }),
      create_number_expr((count - index - 1) as f64),
    )
  };

  let mut joined = shifted(0, first);

  for (index, condition) in rest.iter().enumerate() {
    joined = create_bin_expr(BinaryOp::BitOr, joined, shifted(index + 1, condition));
  }

  Box::new(joined)
}

fn gen_condition_permutations(count: usize) -> Vec<Vec<bool>> {
  (0..2usize.pow(count as u32))
    .map(|i| (0..count).map(|j| i & (1 << j) != 0).collect())
    .collect()
}

#[cfg(test)]
#[path = "tests/make_string_expression_tests.rs"]
mod tests;
