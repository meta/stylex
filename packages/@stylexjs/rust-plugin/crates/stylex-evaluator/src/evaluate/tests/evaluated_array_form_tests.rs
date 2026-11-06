//! The array literal an evaluated list is written as.
//!
//! An evaluated array has two spellings -- the evaluator's own list of values,
//! and the literal a reader needs. `evaluate_result_vec_to_array_expr` is the
//! one place that turns the first into the second, and it answers `None` for a
//! list that has no literal form rather than a shorter array: an element
//! silently dropped writes a value the source does not describe.
//!
//! Two sides, because answering `None` for every list would satisfy the first
//! half alone. Each refusal is paired with the list beside it that has to keep
//! its literal form.
//!
//! Asked of the function directly as well as through source. The transform
//! above calls it with values this crate did not build, and the `env` option's
//! napi bridge writes the same lists. So the whitelist the doc states is a
//! contract of the function, not of one source shape.

use super::source_evaluation::*;
use super::*;
use stylex_ast::ast::convertors::create_null_expr;
use stylex_constants::constants::messages::{
  ILLEGAL_PROP_ARRAY_VALUE, SPREAD_PROPERTIES_UNREADABLE,
};

/// The four expressions that can stand as an element value, one written array
/// each. This is the pair the refusals below are measured against.
///
/// Each row says what it wrote rather than how many things it wrote. A form
/// that kept the count and lost the nesting -- an array of arrays flattened to
/// one -- passes a count and is a value the source does not describe.
#[test]
fn the_four_element_shapes_keep_their_literal_form() {
  assert_element_shapes("[1, 'a', true, null]", &["1", "'a'", "true", "null"]);
  assert_element_shapes("[[1], [[2]]]", &["[1]", "[[2]]"]);
  assert_element_shapes("[{ a: 1 }]", &["{a:1}"]);
  assert_element_shapes("[undefined]", &["undefined"]);
  assert_element_shapes("[]", &[]);
}

/// Every array this producer writes carries one present element per slot and
/// no spread, at every level.
///
/// The readers of an evaluator-written array rely on that and none of them
/// re-checks it: the count of slots is the element count, and a reader takes
/// the element out of the slot without asking whether one is there. So a hole
/// or a spread written here is a fault they inherit rather than answer for --
/// which is why it is asserted of the producer.
#[test]
fn every_written_array_carries_one_present_element_per_slot() {
  for source in [
    "[1, 'a', true, null]",
    "[[1], [[2]]]",
    "[{ a: 1 }, { b: [2] }]",
    "[undefined]",
    "[]",
  ] {
    match evaluate_result_vec_to_array_expr(&folds_to_a_list(source)) {
      Some(form) => assert_written_form(&form, source),
      None => panic!("expected `{}` to have an array literal form", source),
    }
  }

  // The two shapes that would write a hole or a spread never become a value at
  // all, which is the other half of the invariant: it holds because the walk
  // refuses them, not because the writer skips them. Read through the deopt
  // rather than through the writer, which is never handed either.
  assert_deopts("[...['a']]");
  assert_deopts("[, 'a']");
}

/// A value the evaluator holds and writes no expression for has no element
/// form, so the array it sits in has none either. An internal map and a folded
/// function are two of them; the third, a callback, is the one an author
/// reaches through the source cases below.
#[test]
fn a_value_with_no_expression_form_has_no_element_form() {
  assert!(
    evaluate_result_vec_to_array_expr(&[EvaluateResultValue::Map(IndexMap::default())]).is_none()
  );
  assert!(
    evaluate_result_vec_to_array_expr(&[EvaluateResultValue::FunctionConfig(FunctionConfig {
      fn_ptr: FunctionType::Mapper(Rc::new(create_null_expr)),
      takes_path: false,
    })])
    .is_none()
  );
}

/// An expression the evaluator does hold, but which no array element can
/// carry. An arrow is the one a fold produces -- it is what a folded function
/// map stands for -- and a call and a property read are the two shapes the
/// readers of this literal treat as "not a value yet".
#[test]
fn an_expression_that_is_not_an_element_shape_has_no_element_form() {
  for expr in [
    fold_placeholder_function(),
    parse_expr("f()"),
    parse_expr("a.b"),
  ] {
    assert!(
      evaluate_result_vec_to_array_expr(&[EvaluateResultValue::Expr(expr.clone())]).is_none(),
      "expected {:?} to have no element form",
      expr
    );
  }
}

/// A nested list carries its own answer up. The outer array has no literal
/// form when the inner one has none, which is the reading that keeps an array
/// of arrays from folding one element short.
#[test]
fn a_nested_list_with_no_form_takes_the_outer_one_with_it() {
  let inner =
    EvaluateResultValue::Vec(vec![EvaluateResultValue::Expr(fold_placeholder_function())]);

  assert!(evaluate_result_vec_to_array_expr(std::slice::from_ref(&inner)).is_none());
  assert!(
    evaluate_result_vec_to_array_expr(&[
      EvaluateResultValue::Expr(create_number_expr(1.0)),
      inner.clone()
    ])
    .is_none(),
    "the refusal travels past the elements that do have a form"
  );
  assert!(
    evaluate_result_vec_to_array_expr(&[EvaluateResultValue::Vec(vec![EvaluateResultValue::Vec(
      vec![inner]
    )])])
    .is_none(),
    "and past however many lists are wrapped around it"
  );
}

/// The same nesting written as source. A style value and a spread operand are
/// the two positions that read a folded list through its literal form, so a
/// callback nested one array down is how an author reaches the refusal above.
#[test]
fn a_style_value_and_a_spread_refuse_a_nested_callback() {
  assert_deopt_reason_contains("({ a: [[() => 1]] })", ILLEGAL_PROP_ARRAY_VALUE);
  assert_deopt_reason_contains("({ a: [[[() => 1]]] })", ILLEGAL_PROP_ARRAY_VALUE);
  assert_deopt_reason_contains("({ ...[[() => 1]] })", SPREAD_PROPERTIES_UNREADABLE);
}

/// And the fold beside each: the same two positions over lists that do have a
/// literal form keep every slot the source wrote.
#[test]
fn a_style_value_and_a_spread_keep_a_nested_list_of_values() {
  assert_folds_to_object_keys("({ a: [[1]], b: [[2], [3]] })", &["a", "b"]);
  assert_folds_to_object_keys("({ ...[[1], [2]] })", &["0", "1"]);
}

// ==================== helpers ====================

/// Asserts the source folds to a list whose literal form spells these
/// elements, in order.
///
/// The elements rather than their count, because a form that keeps the count
/// and changes what is in it is the answer this file is about.
#[track_caller]
fn assert_element_shapes(source: &str, expected: &[&str]) {
  match evaluate_result_vec_to_array_expr(&folds_to_a_list(source)) {
    Some(Expr::Array(array)) => {
      let written = array
        .elems
        .iter()
        .map(|elem| match elem {
          Some(elem) => printed(&elem.expr),
          None => panic!("expected `{}` to write no hole", source),
        })
        .collect::<Vec<String>>();

      assert_eq!(written, expected, "wrong element form for `{}`", source);
    },
    other => panic!(
      "expected `{}` to have an array literal form, got {:?}",
      source, other
    ),
  }
}
