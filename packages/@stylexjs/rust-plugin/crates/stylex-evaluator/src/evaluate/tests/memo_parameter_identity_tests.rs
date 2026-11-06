//! Distinct retained callback binding maps must never share a memo answer.
use super::*;
use stylex_ast::ast::factories::create_ident;
use stylex_state::functions::{FunctionConfig, FunctionConfigType, FunctionType};

fn parameter(value: f64) -> Box<FunctionConfigType> {
  Box::new(FunctionConfigType::Regular(FunctionConfig {
    fn_ptr: FunctionType::ValueMapper(Rc::new(move || {
      EvaluateResultValue::Expr(create_number_expr(value))
    })),
    takes_path: false,
  }))
}
fn map(a: &FunctionConfigType, b: &FunctionConfigType) -> FunctionMap {
  let mut functions = FunctionMap::default();
  functions
    .bindings
    .insert(create_ident("total").to_id(), Box::new(a.clone()));
  functions
    .bindings
    .insert(create_ident("x").to_id(), Box::new(b.clone()));
  functions
}
#[test]
fn memo_distinct_retained_parameter_contexts_do_not_reuse_a_wrong_value() {
  // Keep allocations alive to isolate aggregate identity collisions from address recycling.
  let parameters: Vec<_> = (0..32).map(|n| parameter(n as f64)).collect();
  let mut collision = None;
  for a in 0..parameters.len() {
    for b in a + 1..parameters.len() {
      let first = Rc::new(map(&parameters[a], &parameters[b]));
      let second = Rc::new(map(&parameters[b], &parameters[a]));
      if first.memo_fingerprint() == second.memo_fingerprint() {
        collision = Some((first, second, a as f64, b as f64));
        break;
      }
    }
    if collision.is_some() {
      break;
    }
  }
  let (first, second, a, b) = collision.expect("retained pointer-sum permutation collision");
  assert_ne!(first, second);
  let expr = Expr::Bin(swc_core::ecma::ast::BinExpr {
    span: swc_core::common::DUMMY_SP,
    op: swc_core::ecma::ast::BinaryOp::Sub,
    left: Box::new(Expr::Ident(create_ident("total"))),
    right: Box::new(Expr::Ident(create_ident("x"))),
  });
  let mut traversal = StateManager::default();
  let first_result = evaluate_with_functions(&expr, &mut traversal, first);
  let second_result = evaluate_with_functions(&expr, &mut traversal, second);
  for (result, expected) in [(first_result, a - b), (second_result, b - a)] {
    assert!(result.confident, "{:?}", result.reason);
    let Some(EvaluateResultValue::Expr(Expr::Lit(Lit::Num(number)))) = result.value else {
      panic!("numeric difference")
    };
    assert_eq!(
      number.value, expected,
      "retained parameter-map collision reused another invocation's value"
    );
  }
}
