use super::*;

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
