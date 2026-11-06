//! Tests literal materialization of evaluated lists and unsupported nested values.

use super::source_evaluation::*;
use super::*;
use stylex_ast::ast::convertors::create_null_expr;
use stylex_constants::constants::messages::{
  ILLEGAL_PROP_ARRAY_VALUE, SPREAD_PROPERTIES_UNREADABLE,
};

#[test]
fn the_four_element_shapes_keep_their_literal_form() {
  assert_element_shapes("[1, 'a', true, null]", &["1", "'a'", "true", "null"]);
  assert_element_shapes("[[1], [[2]]]", &["[1]", "[[2]]"]);
  assert_element_shapes("[{ a: 1 }]", &["{a:1}"]);
  assert_element_shapes("[undefined]", &["undefined"]);
  assert_element_shapes("[]", &[]);
}

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

  assert_deopts("[, 'a']");
  assert_deopts("[...1]");

  for spread in ["[...['a'], 'b']", "[...'ab']"] {
    match evaluate_result_vec_to_array_expr(&folds_to_a_list(spread)) {
      Some(form) => assert_written_form(&form, spread),
      None => panic!("expected `{}` to have an array literal form", spread),
    }
  }
}

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

#[test]
fn a_style_value_and_a_spread_refuse_a_nested_callback() {
  assert_deopt_reason_contains("({ a: [[() => 1]] })", ILLEGAL_PROP_ARRAY_VALUE);
  assert_deopt_reason_contains("({ a: [[[() => 1]]] })", ILLEGAL_PROP_ARRAY_VALUE);
  assert_deopt_reason_contains("({ ...[[() => 1]] })", SPREAD_PROPERTIES_UNREADABLE);
}

#[test]
fn a_style_value_and_a_spread_keep_a_nested_list_of_values() {
  assert_folds_to_object_keys("({ a: [[1]], b: [[2], [3]] })", &["a", "b"]);
  assert_folds_to_object_keys("({ ...[[1], [2]] })", &["0", "1"]);
}

#[test]
fn a_spread_keeps_a_callback_that_is_the_entry_itself() {
  assert_folds_to_string("Object.keys({ ...[() => 1] }).join(',')", "0");
  assert_folds_to_string("typeof ({ ...[() => 1] })[0]", "function");
}

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
