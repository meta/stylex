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
  // `void x` is `undefined` whatever `x` is, so the operand is never evaluated
  // and an operand that would have deopted cannot deopt this.
  //
  // Answered as the `undefined` identifier rather than as no value: the
  // evaluator's caller turns a confident `None` into a deopt, so returning
  // nothing here would fail the build on an expression that has a value.
  if unary.op == UnaryOp::Void {
    return Some(js_undefined());
  }

  let argument = &unary.arg;

  // Answered without folding the operand: `typeof` never evaluates a function,
  // and folding one would answer for its body rather than for its kind. All
  // three spellings, because an arrow is neither of the other two -- it used to
  // fall through, fold to a callback, and then be refused for having no
  // expression form, where the language says `"function"`. Read through the
  // parentheses a function expression in a value position has to be written
  // with, which the fold unwraps but this check runs ahead of.
  if unary.op == UnaryOp::TypeOf {
    let bare = normalize_expr(argument);

    if bare.is_fn_expr() || bare.is_class() || bare.is_arrow() {
      return Some(EvaluateResultValue::Expr(create_string_expr("function")));
    }
  }

  // One question rather than two. An operand that answers nothing is an operand
  // with no value to read a kind or a number off, whether it recorded its own
  // refusal or came back empty out of the memo -- a memo answers `None`
  // confidently, so the `?` is not standing in for a value guard here. The
  // absence is carried up and the dispatch above names the node.
  //
  // `typeof someObject.method` is ordinary JavaScript, and what it reads is the
  // member's own refusal rather than one this node invents.
  let arg = evaluate_cached(argument, state, traversal_state, fns)?;

  // `!` is answered off the evaluated value rather than off an expression form
  // of it, and through the one `ToBoolean` bridge the logical operators read.
  // An operand with no expression form still has a truthiness -- the
  // evaluator's own array and function-map spellings all stand for objects,
  // which are truthy -- so `![]` folds where reading it as an expression
  // refused. The bridge's refusal is the operand whose *kind* cannot be read,
  // and that deopts.
  if unary.op == UnaryOp::Bang {
    let Some(value) = evaluate_result_to_js_boolean(&arg) else {
      deopt_unsupported!(deopt, &create_unary_expr(unary), state, ILLEGAL_PROP_VALUE);
    };

    return Some(EvaluateResultValue::Expr(create_bool_expr(!value)));
  }

  // `typeof` reads the operand's *kind*, which every evaluated value has,
  // rather than a primitive out of it -- so it is answered off the value like
  // `!` is, and before the expression-form guard below.
  if unary.op == UnaryOp::TypeOf {
    let Some(arg_type) = type_of(&arg) else {
      // A value with no kind to read. What reaches this is a value that is not
      // there: a fold answered nothing for it while staying confident, and the
      // array it sat in kept the slot. The sentence is the one `!` gives for
      // the same value, because it is the same complaint -- the operand has no
      // reading at all, rather than a shape that could be named.
      deopt_unsupported!(deopt, &create_unary_expr(unary), state, ILLEGAL_PROP_VALUE);
    };

    return Some(EvaluateResultValue::Expr(create_string_expr(arg_type)));
  }

  match unary.op {
    UnaryOp::Plus => evaluate_unary_numeric_of(unary, &arg, state, traversal_state, fns, |v| v),
    UnaryOp::Minus => evaluate_unary_numeric_of(unary, &arg, state, traversal_state, fns, |v| -v),
    // `~` applies `ToInt32` first, so the negation happens in 32 bits: JS says
    // `~[4294967296]` is `-1`, where a 64-bit negation says `-4294967297`. The
    // operands that reach the wrap are the ones the number bridge newly made
    // reachable, an array or an object whose string form is a large number.
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

/// The type name `typeof` answers for an evaluated value, or `None` where the
/// value's kind cannot be read.
///
/// The expression arm is the primitive table; every variant the evaluator has of
/// its own stands for an object or a function upstream, which is the same
/// classification the `ToObject` bridge makes, so it is asked rather than
/// restated.
fn type_of(value: &EvaluateResultValue) -> Option<&'static str> {
  let EvaluateResultValue::Expr(expr) = value else {
    // Every value the evaluator has of its own stands for an object or a
    // function upstream, and the one `ToObject` bridge decides which. It
    // refuses the absent element of an array, whose meaning that bridge
    // deliberately leaves undecided, and the refusal travels out from here.
    return match evaluate_result_to_js_object(value)? {
      coercions::ObjectCoercion::Function => Some("function"),
      _ => Some("object"),
    };
  };

  match expr {
    Expr::Lit(Lit::Str(_)) => Some("string"),
    Expr::Lit(Lit::Bool(_)) => Some("boolean"),
    Expr::Lit(Lit::Num(_)) => Some("number"),
    // A big integer is a primitive of its own, and the only value whose kind
    // the `ToObject` bridge below would name `object` wrongly -- every other
    // literal it reads really is one. No source reaches it, because both
    // compilers refuse a big-integer literal before the value walk sees it; the
    // arm starts to matter the day the dispatch admits one, which is the same
    // day the bridge would begin answering `object` for it.
    Expr::Lit(Lit::BigInt(_)) => Some("bigint"),
    Expr::Ident(ident) if is_js_undefined(ident) => Some("undefined"),
    // Every other kind an evaluated value holds is an object or a function
    // upstream, and the same bridge decides which: `null` is the object
    // `typeof` names, an array and an object literal are objects, and the three
    // function spellings are functions.
    //
    // `None` is a kind this evaluator has no reading of. It cannot arrive from
    // a fold -- the dispatch refuses every expression kind the table above and
    // the bridge do not cover -- so what it guards is a caller asking about a
    // value the evaluator never made.
    _ => match coercions::to_object(expr)? {
      coercions::ObjectCoercion::Function => Some("function"),
      _ => Some("object"),
    },
  }
}

/// `ToNumber` over an evaluated operand, then the operator's arithmetic.
///
/// [`operand_to_js_number`] reads the operand. Requiring only `expr_to_num`
/// refused `-({})`, `+({})`, `~({})` and `-[1, 2, 3]`, all four of which
/// upstream folds.
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
