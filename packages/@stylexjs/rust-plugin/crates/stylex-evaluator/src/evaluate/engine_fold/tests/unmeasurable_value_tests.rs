//! The readings that answer nothing, over the values and the budget no walk
//! hands them.
//!
//! Each is a step that cannot fail from what reaches it. A literal the walk
//! admits always has a number; a value the reader answers with always writes an
//! expression down, because the walk resolves a name only where it is about to
//! carry it and none of this compiler's own values crosses; and the width walk
//! is handed a budget the evaluator has already measured the same value
//! against, so it never runs out first.
//!
//! They are kept rather than deleted because each is what stops a bound being
//! read off a value that has none — which is the difference between refusing a
//! call and folding one that builds more than the ceiling allows.

use super::*;

use stylex_ast::ast::convertors::{create_ident_expr, create_number_expr, create_string_expr};
use stylex_ast::ast::factories::create_array_expression;
use stylex_state::theme_ref::ThemeRef;
use swc_core::common::DUMMY_SP;
use swc_core::ecma::ast::{ExprOrSpread, ObjectLit, PropOrSpread, Regex, SpreadElement};

/// A regular expression written down, which reads its number through its own
/// text like every other literal.
fn a_regex() -> Expr {
  Expr::Lit(Lit::Regex(Regex {
    span: DUMMY_SP,
    exp: "a".into(),
    flags: "".into(),
  }))
}

/// An object holding keys this compiler cannot name, which is the shape with no
/// compile-time number at all: a spread stands for keys nothing here reads, and
/// one of them may be the method the conversion would have asked for.
fn an_object_spreading_a_name() -> Expr {
  Expr::Object(ObjectLit {
    span: DUMMY_SP,
    props: vec![PropOrSpread::Spread(SpreadElement {
      dot3_token: DUMMY_SP,
      expr: Box::new(create_ident_expr("rest")),
    })],
  })
}

/// A value this compiler holds of its own, which writes no expression down.
fn a_theme_group() -> EvaluateResultValue {
  EvaluateResultValue::ThemeRef(ThemeRef::new("vars.stylex.js", "vars", "x"))
}

/// A written literal bounds a call at the number the language reads out of it,
/// and at nothing where it has none.
#[test]
fn a_written_literal_bounds_a_call_at_its_own_number() {
  assert_eq!(count_written(&create_number_expr(3.0)), Some(3));
  // The language's own reading, which is why a count is not required to be
  // written as a number.
  assert_eq!(count_written(&create_string_expr("3")), Some(3));
  assert_eq!(count_written(&create_string_expr("lots")), Some(0));
  // A regular expression is text like any other here, and its text is no
  // number — which bounds the call at nothing built rather than refusing it.
  assert_eq!(count_written(&a_regex()), Some(0));

  assert_eq!(
    count_written(&an_object_spreading_a_name()),
    None,
    "a value with no compile-time number must bound the call at nothing rather than at zero"
  );
}

/// A resolved value is read through the expression it writes down, and a value
/// that writes none bounds nothing.
#[test]
fn a_resolved_value_bounds_a_call_through_what_it_writes() {
  assert_eq!(
    count_resolved(&EvaluateResultValue::Expr(create_number_expr(2.0))),
    Some(2)
  );

  assert_eq!(
    count_resolved(&a_theme_group()),
    None,
    "a value that writes no expression must bound the call at nothing"
  );
}

/// The narrower reading answers only for a number that was written, and answers
/// nothing for a value that writes no expression at all.
#[test]
fn the_written_number_of_a_resolved_value_is_narrower() {
  assert_eq!(
    number_resolved(&EvaluateResultValue::Expr(create_number_expr(2.0))),
    Some(2)
  );

  // Text that coerces to a number is not a written one: the arithmetic above
  // this reading is sound only over numbers the guard has seen.
  assert_eq!(
    number_resolved(&EvaluateResultValue::Expr(create_string_expr("2"))),
    None
  );

  assert_eq!(number_resolved(&a_theme_group()), None);
}

/// The width walk answers nothing where it has no room to descend, in both the
/// shapes an array reaches it as.
///
/// One level of nesting is one descent, so a budget of none stops the walk at
/// the first array it meets — the list the evaluator built and the literal the
/// author wrote alike.
#[test]
fn the_width_walk_answers_nothing_with_no_room_to_descend() {
  let nothing = Depth::full(0);
  let list = EvaluateResultValue::Vec(vec![EvaluateResultValue::Expr(create_string_expr("a"))]);
  let written = an_array_of(&["a"]);

  assert_eq!(rendered_characters(&list, nothing), None);
  assert_eq!(rendered_expr(&written, nothing), None);
}

/// With room, both shapes answer the same width for the same array, which is
/// what says the two readings measure one value rather than two.
#[test]
fn both_shapes_of_an_array_measure_the_same_width() {
  let room = Depth::full(8);
  let list = EvaluateResultValue::Vec(vec![
    EvaluateResultValue::Expr(create_string_expr("ab")),
    EvaluateResultValue::Expr(create_string_expr("c")),
  ]);
  let written = an_array_of(&["ab", "c"]);

  // Two elements and the comma the language joins them with.
  assert_eq!(rendered_characters(&list, room), Some(4));
  assert_eq!(rendered_expr(&written, room), Some(4));
}

/// A slot with no expression to measure stops the width walk rather than being
/// stepped over.
///
/// A hole and a spread both read a bound short -- a hole would be skipped and a
/// spread measured as its operand, where it stands for a count the source does
/// not state -- and this is the one guard whose being wrong costs unbounded
/// memory rather than a wrong declaration. No producer writes either into an
/// [evaluator-written array](../../../../CONTEXT.md#evaluator-written-array), so
/// the reading is asked here rather than through a source.
#[test]
fn a_slot_with_no_expression_stops_the_width_walk() {
  let room = Depth::full(8);

  let with_a_hole = create_array_expression(vec![
    Some(ExprOrSpread {
      spread: None,
      expr: Box::new(create_string_expr("ab")),
    }),
    None,
  ]);

  let with_a_spread = create_array_expression(vec![Some(ExprOrSpread {
    spread: Some(DUMMY_SP),
    expr: Box::new(create_array_expression(vec![])),
  })]);

  assert_eq!(rendered_expr(&with_a_hole, room), None);
  assert_eq!(rendered_expr(&with_a_spread, room), None);
}

/// An array literal holding `texts`, which is the shape an author writes.
fn an_array_of(texts: &[&str]) -> Expr {
  create_array_expression(
    texts
      .iter()
      .map(|text| {
        Some(ExprOrSpread {
          spread: None,
          expr: Box::new(create_string_expr(text)),
        })
      })
      .collect(),
  )
}
