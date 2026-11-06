use super::*;
use stylex_ast::ast::convertors::{create_ident_expr, create_null_expr};
use swc_core::common::DUMMY_SP;

fn bin(op: BinaryOp, left: Expr, right: Expr) -> BinExpr {
  BinExpr {
    span: DUMMY_SP,
    op,
    left: Box::new(left),
    right: Box::new(right),
  }
}

fn fold_expr(op: BinaryOp, left: Expr, right: Expr) -> (Option<Expr>, Option<String>) {
  let mut state = EvaluationState::new();
  let mut traversal_state = StateManager::default();
  let fns = FunctionMap::default();

  let logical_op = match LogicalOp::of(op) {
    Some(logical_op) => logical_op,
    None => panic!("{:?} is not a logical operator", op),
  };

  let result = evaluate(
    logical_op,
    &bin(op, left, right),
    &mut state,
    &mut traversal_state,
    &fns,
  );

  let value = result.and_then(|value| value.as_expr().cloned());

  (value, state.deopt_reason)
}

fn folded_string(op: BinaryOp, left: Expr, right: Expr) -> String {
  match fold_expr(op, left, right) {
    (Some(Expr::Lit(Lit::Str(strng))), _) => convert_atom_to_string(&strng.value),
    (value, reason) => panic!("expected a folded string, got {:?} ({:?})", value, reason),
  }
}

#[test]
fn nullish_takes_the_right_side_for_null() {
  assert_eq!(
    folded_string(
      BinaryOp::NullishCoalescing,
      create_null_expr(),
      create_string_expr("red"),
    ),
    "red"
  );
}

#[test]
fn nullish_takes_the_right_side_for_undefined() {
  assert_eq!(
    folded_string(
      BinaryOp::NullishCoalescing,
      create_ident_expr("undefined"),
      create_string_expr("red"),
    ),
    "red"
  );
}

#[test]
fn nullish_keeps_a_left_side_that_is_neither() {
  assert_eq!(
    folded_string(
      BinaryOp::NullishCoalescing,
      create_string_expr("blue"),
      create_string_expr("red"),
    ),
    "blue"
  );
}

/// Babel's inherited `??` guard checks truthiness, not nullishness.
#[test]
fn nullish_refuses_a_falsy_left_side_that_is_not_nullish() {
  for left in [
    create_number_expr(0.0),
    create_bool_expr(false),
    create_string_expr(""),
  ] {
    let (value, reason) = fold_expr(
      BinaryOp::NullishCoalescing,
      left.clone(),
      create_string_expr("red"),
    );

    assert!(value.is_none(), "expected {:?} ?? 'red' to refuse", left);
    assert_eq!(reason.as_deref(), Some("unknown error"));
  }
}

#[test]
fn or_takes_the_left_side_when_it_is_truthy() {
  assert_eq!(
    folded_string(
      BinaryOp::LogicalOr,
      create_string_expr("blue"),
      create_string_expr("red"),
    ),
    "blue"
  );
}

#[test]
fn or_takes_the_right_side_when_the_left_is_falsy() {
  assert_eq!(
    folded_string(
      BinaryOp::LogicalOr,
      create_string_expr(""),
      create_string_expr("red"),
    ),
    "red"
  );
}

#[test]
fn and_takes_the_right_side_when_the_left_is_truthy() {
  assert_eq!(
    folded_string(
      BinaryOp::LogicalAnd,
      create_string_expr("blue"),
      create_string_expr("red"),
    ),
    "red"
  );
}

#[test]
fn and_returns_a_falsy_left_side_verbatim() {
  assert_eq!(
    folded_string(
      BinaryOp::LogicalAnd,
      create_string_expr(""),
      create_string_expr("red"),
    ),
    ""
  );
}

#[test]
fn the_winning_operand_keeps_its_own_shape() {
  let (value, reason) = fold_expr(
    BinaryOp::NullishCoalescing,
    create_null_expr(),
    Expr::Object(create_object_lit(vec![])),
  );

  assert!(
    matches!(value, Some(Expr::Object(_))),
    "expected the object operand back, got {:?} ({:?})",
    value,
    reason
  );
}

#[test]
fn an_unresolvable_right_side_does_not_deopt_a_decided_or() {
  assert_eq!(
    folded_string(
      BinaryOp::LogicalOr,
      create_string_expr("blue"),
      create_ident_expr("unknown"),
    ),
    "blue"
  );
}

#[test]
fn an_unresolvable_right_side_deopts_an_undecided_or() {
  let (value, reason) = fold_expr(
    BinaryOp::LogicalOr,
    create_string_expr(""),
    create_ident_expr("unknown"),
  );

  assert!(value.is_none(), "expected the expression to deopt");
  assert!(reason.is_some(), "expected a deopt reason");
}

#[test]
fn only_the_three_logical_operators_are_recognised() {
  assert_eq!(LogicalOp::of(BinaryOp::LogicalOr), Some(LogicalOp::Or));
  assert_eq!(LogicalOp::of(BinaryOp::LogicalAnd), Some(LogicalOp::And));
  assert_eq!(
    LogicalOp::of(BinaryOp::NullishCoalescing),
    Some(LogicalOp::Nullish)
  );

  for op in [BinaryOp::Add, BinaryOp::Sub, BinaryOp::EqEqEq, BinaryOp::In] {
    assert!(
      LogicalOp::of(op).is_none(),
      "expected {:?} not to be a logical operator",
      op
    );
  }
}

#[test]
fn the_right_operand_runs_only_where_the_language_reaches_it() {
  let truthy = EvaluateResultValue::Expr(create_string_expr("red"));
  let falsy = EvaluateResultValue::Expr(create_string_expr(""));
  let zero = EvaluateResultValue::Expr(create_number_expr(0.0));
  let null = EvaluateResultValue::Expr(create_null_expr());

  for (op, left, runs, why) in [
    (
      LogicalOp::And,
      &truthy,
      true,
      "`x && y` reaches y for a truthy x",
    ),
    (
      LogicalOp::And,
      &falsy,
      false,
      "`x && y` skips y for a falsy x",
    ),
    (
      LogicalOp::Or,
      &truthy,
      false,
      "`x || y` skips y for a truthy x",
    ),
    (
      LogicalOp::Or,
      &falsy,
      true,
      "`x || y` reaches y for a falsy x",
    ),
    (
      LogicalOp::Nullish,
      &truthy,
      false,
      "`x ?? y` skips y for a set x",
    ),
    (
      LogicalOp::Nullish,
      &null,
      true,
      "`x ?? y` reaches y for null",
    ),
    (LogicalOp::Nullish, &zero, true, "`0 ?? y` decides nothing"),
    (LogicalOp::And, &zero, false, "`0 && y` skips y"),
    (LogicalOp::Or, &zero, true, "`0 || y` reaches y"),
  ] {
    assert_eq!(evaluates_its_right_operand(op, left), runs, "{}", why);
  }
}

#[test]
fn an_undefined_left_side_is_read_as_the_falsy_nullish_value_it_is() {
  let undefined = js_undefined();

  assert!(!evaluates_its_right_operand(LogicalOp::And, &undefined));
  assert!(evaluates_its_right_operand(LogicalOp::Or, &undefined));
  assert!(evaluates_its_right_operand(LogicalOp::Nullish, &undefined));
}

#[test]
fn a_left_side_with_no_value_at_all_reads_as_undefined() {
  assert_eq!(truthiness(None), Some(false), "no value is falsy");
  assert!(is_nullish(None), "no value is nullish");

  assert!(
    matches!(decide(LogicalOp::And, None), Decision::Left),
    "`&& y` keeps the falsy left side"
  );
  assert!(
    matches!(decide(LogicalOp::Or, None), Decision::Right),
    "`|| y` reaches y past a falsy left side"
  );
  assert!(
    matches!(decide(LogicalOp::Nullish, None), Decision::Right),
    "`?? y` reaches y past a nullish left side"
  );
}

#[test]
fn a_left_side_with_no_truthiness_refuses_the_two_operators_that_need_one() {
  let unreadable = EvaluateResultValue::Null;

  assert_eq!(truthiness(Some(&unreadable)), None, "it has no truthiness");

  assert!(
    matches!(decide(LogicalOp::Or, Some(&unreadable)), Decision::Refuse),
    "`|| y` cannot choose"
  );
  assert!(
    matches!(decide(LogicalOp::And, Some(&unreadable)), Decision::Refuse),
    "`&& y` cannot choose"
  );
  assert!(
    matches!(
      decide(LogicalOp::Nullish, Some(&unreadable)),
      Decision::Right
    ),
    "`?? y` reaches y, because absence is what it asks about"
  );
}
