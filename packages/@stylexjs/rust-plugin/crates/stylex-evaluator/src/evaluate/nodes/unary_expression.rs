use super::super::*;
use stylex_ast::ast::factories::create_unary_expr;
use stylex_macros::deopt_unsupported;
use swc_core::ecma::ast::{Lit, UnaryExpr, UnaryOp};

pub(in super::super) fn evaluate(
  unary: &UnaryExpr,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Option<EvaluateResultValue> {
  // `void x` is `undefined` whatever `x` is, so the operand is never evaluated and an operand that would have deopted cannot deopt this.
  if unary.op == UnaryOp::Void {
    return Some(js_undefined());
  }

  let argument = &unary.arg;

  if unary.op == UnaryOp::TypeOf {
    let bare = normalize_expr(argument);

    if bare.is_fn_expr() || bare.is_class() || bare.is_arrow() {
      return Some(EvaluateResultValue::Expr(create_string_expr("function")));
    }
  }

  let mut arg = evaluate_cached(argument, state, traversal_state, fns)?;
  if matches!(unary.op, UnaryOp::Plus | UnaryOp::Minus | UnaryOp::Tilde)
    && matches!(
      arg,
      EvaluateResultValue::Object(_)
        | EvaluateResultValue::SharedArray(_)
        | EvaluateResultValue::NativeArray(_)
        | EvaluateResultValue::Vec(_)
        | EvaluateResultValue::ThemeRef(_)
        | EvaluateResultValue::Expr(Expr::Object(_) | Expr::Array(_))
    )
  {
    arg = match super::super::primitive::primitive(
      &arg,
      stylex_structures::stylex_env::PrimitiveHint::Number,
      traversal_state,
      unary.span,
    ) {
      Ok(value) => value,
      Err(error) => {
        if let Some(refusal) = stylex_js::native::quota_refusal() {
          return deopt(
            &create_unary_expr(unary),
            state,
            &quota_reason(refusal, QuotaBoundary::Growth(NUMERIC_CONVERSION)),
          );
        }
        return super::super::callback_failure::invocation_failure(error, state);
      },
    };
  }
  if matches!(arg, EvaluateResultValue::Host(_)) {
    return match super::super::host_operators::unary_value(unary.op, &arg, traversal_state) {
      Ok(value) => Some(value),
      Err(error) => deopt(&create_unary_expr(unary), state, &error.message),
    };
  }

  if unary.op == UnaryOp::Bang {
    let Some(value) = evaluate_result_to_js_boolean(&arg) else {
      deopt_unsupported!(deopt, &create_unary_expr(unary), state, ILLEGAL_PROP_VALUE);
    };

    return Some(EvaluateResultValue::Expr(create_bool_expr(!value)));
  }

  if unary.op == UnaryOp::TypeOf {
    let Some(arg_type) = type_of(&arg) else {
      deopt_unsupported!(deopt, &create_unary_expr(unary), state, ILLEGAL_PROP_VALUE);
    };

    return Some(EvaluateResultValue::Expr(create_string_expr(arg_type)));
  }

  match unary.op {
    UnaryOp::Plus => evaluate_unary_numeric_of(unary, &arg, state, traversal_state, fns, |v| v),
    UnaryOp::Minus => evaluate_unary_numeric_of(unary, &arg, state, traversal_state, fns, |v| -v),

    UnaryOp::Tilde => evaluate_unary_numeric_of(unary, &arg, state, traversal_state, fns, |v| {
      f64::from(!coercions::to_int32(v))
    }),
    _ => deopt(
      &create_unary_expr(unary),
      state,
      &unsupported_operator(unary.op.as_str()),
    ),
  }
}

/// The type name `typeof` answers for an evaluated value, or `None` where the value's kind cannot be read.
fn type_of(value: &EvaluateResultValue) -> Option<&'static str> {
  let EvaluateResultValue::Expr(expr) = value else {
    // Every value the evaluator has of its own stands for an object or a function upstream, and the one `ToObject` bridge decides which.
    return match evaluate_result_to_js_object(value)? {
      coercions::ObjectCoercion::Function => Some("function"),
      _ => Some("object"),
    };
  };

  match expr {
    Expr::Lit(Lit::Str(_)) => Some("string"),
    Expr::Lit(Lit::Bool(_)) => Some("boolean"),
    Expr::Lit(Lit::Num(_)) => Some("number"),

    Expr::Lit(Lit::BigInt(_)) => Some("bigint"),
    Expr::Ident(ident) if is_js_undefined(ident) => Some("undefined"),
    // It cannot arrive from a fold -- the dispatch refuses every expression kind the table above and the bridge do not cover -- so what it guards is a caller asking about a value the evaluator never made.
    _ => match coercions::to_object(expr)? {
      coercions::ObjectCoercion::Function => Some("function"),
      _ => Some("object"),
    },
  }
}

/// `ToNumber` over an evaluated operand, then the operator's arithmetic.
fn evaluate_unary_numeric_of(
  unary: &UnaryExpr,
  arg: &EvaluateResultValue,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
  transform: impl FnOnce(f64) -> f64,
) -> Option<EvaluateResultValue> {
  let value = match operand_to_js_number(arg, state, traversal_state, fns) {
    Ok(value) => value,
    Err(refusal) => deopt_unsupported!(
      deopt,
      &create_unary_expr(unary),
      state,
      &refusal.reason(traversal_state.character_ceiling())
    ),
  };

  Some(EvaluateResultValue::Expr(create_number_expr(transform(
    value,
  ))))
}

#[cfg(test)]
#[path = "tests/type_of_tests.rs"]
mod type_of_tests;

#[cfg(test)]
#[path = "tests/unary_operator_tests.rs"]
mod unary_operator_tests;
