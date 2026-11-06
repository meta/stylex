//! Tests global conversions over compiler-held values and coercion limits.

use crate::evaluate::source_evaluation::*;
use stylex_ast::ast::convertors::{
  convert_atom_to_string, convert_key_value_to_str, create_number_expr,
};
use stylex_constants::constants::evaluation_errors::{
  NUMERIC_CONVERSION, SPREAD_ELEMENT, STRING_CONVERSION, uncoercible_value,
};
use stylex_state::evaluate_result_value::EvaluateResultValue;
use swc_core::ecma::ast::Expr;

#[track_caller]
fn assert_refuses_naming(source: &str, global: &str) {
  assert_refused_with(
    &evaluated_against_a_function_fold(source),
    source,
    &uncoercible_value(global),
  );
}

#[track_caller]
fn folded_value(source: &str) -> EvaluateResultValue {
  folded_value_of(evaluated_against_a_function_fold(source), source)
}

#[track_caller]
fn assert_conversion_is_nan(source: &str) {
  assert_result_folds_to_nan(evaluated_against_a_function_fold(source), source);
}

#[test]
fn the_string_conversion_refuses_a_value_with_no_string_form() {
  assert_refuses_naming("String(own)", "String");
}

#[test]
fn the_number_conversion_answers_not_a_number_for_a_function() {
  assert_conversion_is_nan("Number(own)");
}

#[test]
fn the_namespace_coerces_as_the_object_it_is() {
  match folded_value("String(sx)") {
    EvaluateResultValue::Expr(expr) => assert_eq!(
      expr
        .as_lit()
        .and_then(|lit| lit.as_str())
        .map(|text| convert_atom_to_string(&text.value)),
      Some(String::from("[object Object]"))
    ),
    other => panic!("expected the object default, got {:?}", other),
  }

  assert_conversion_is_nan("Number(sx)");
}

#[test]
fn the_object_conversion_hands_a_function_fold_back() {
  assert!(
    matches!(
      folded_value("Object(sx)"),
      EvaluateResultValue::FunctionConfigMap(_)
    ),
    "expected the fold itself back"
  );
}

#[test]
fn the_array_conversion_reads_every_argument_as_an_element() {
  match folded_value("Array(sx)") {
    EvaluateResultValue::Vec(items) => assert_eq!(items.len(), 1),
    other => panic!("expected a one-element array, got {:?}", other),
  }

  match folded_value("Array(sx, sx)") {
    EvaluateResultValue::Vec(items) => assert_eq!(items.len(), 2),
    other => panic!("expected a two-element array, got {:?}", other),
  }
}

#[test]
fn the_surplus_arguments_of_the_other_three_are_ignored() {
  assert_refuses_naming("String(own, 'ignored')", "String");
  assert_conversion_is_nan("Number(own, 'ignored')");
  assert!(
    matches!(
      folded_value("Object(sx, 'ignored')"),
      EvaluateResultValue::FunctionConfigMap(_)
    ),
    "expected the fold itself back"
  );
}

#[test]
fn a_global_that_names_no_conversion_is_not_converted() {
  let result = evaluated_against_a_function_fold("Math(sx)");

  assert!(!result.confident, "expected `Math(sx)` to refuse");
  assert_ne!(
    result.reason.as_deref(),
    Some(uncoercible_value("Math").as_str()),
    "`Math` is not a conversion and must not be refused as one"
  );
}

#[test]
fn the_array_conversion_bounds_an_evaluated_namespace_length() {
  let source = "Array(String(sx).length)";
  let result = evaluated_against_a_function_fold(source);
  assert!(result.confident, "{:?}", result.reason);
  let value = result.value.expect("bounded array value");
  let data = crate::evaluate::materialize_result(
    &value,
    &stylex_state::state_manager::StateManager::default(),
  )
  .expect("pure data boundary")
  .expect("materializable array");
  let Expr::Array(array) = data else {
    panic!("expected array data")
  };
  assert_eq!(array.elems.len(), 15);
  assert!(
    array.elems.iter().all(Option::is_none),
    "constructor slots stay sparse"
  );
  assert!(!evaluated_against_a_function_fold("Array(10001)").confident);
  assert!(!evaluated_against_a_function_fold("Array(unknownLength)").confident);
}

#[test]
fn the_number_conversion_refuses_a_value_with_no_numeric_form() {
  let source = "Number({ toString: 1 }, own)";

  assert_refused_with(
    &evaluated_against_a_function_fold(source),
    source,
    "Cannot convert object to primitive value",
  );
}

#[test]
fn the_string_conversion_refuses_a_text_past_the_ceiling() {
  assert_refused_at_the_character_ceiling(
    4,
    &a_function_fold(),
    STRING_CONVERSION,
    "String(['1234567890'], own)",
  );
}

#[test]
fn the_number_conversion_refuses_a_text_past_the_ceiling() {
  assert_refused_at_the_character_ceiling(
    4,
    &a_function_fold(),
    NUMERIC_CONVERSION,
    "Number(['1234567890'], own)",
  );
}

#[test]
fn the_conversions_answer_an_ordinary_value_on_this_path_too() {
  assert_eq!(
    folded_value("Number(1, own)"),
    EvaluateResultValue::Expr(create_number_expr(1.0))
  );

  match folded_value("Object({ a: 1 }, own)") {
    EvaluateResultValue::Expr(Expr::Object(object)) => assert_eq!(
      object
        .props
        .iter()
        .filter_map(|prop| prop.as_prop())
        .filter_map(|prop| prop.as_key_value())
        .map(convert_key_value_to_str)
        .collect::<Vec<String>>(),
      vec![String::from("a")]
    ),
    other => panic!("expected the object itself, got {:?}", other),
  }
}

#[test]
fn a_spread_argument_refuses_before_any_conversion() {
  for source in [
    "String(...own)",
    "Number(...own)",
    "Object(...own)",
    "Array(...own)",
  ] {
    let result = evaluated_against_a_function_fold(source);

    assert_refused(&result, source);

    assert_eq!(
      result.reason.as_deref(),
      Some(SPREAD_ELEMENT),
      "a spread must refuse as a spread rather than as a conversion"
    );
  }
}

#[test]
fn an_argument_that_refuses_stops_the_conversion() {
  let source = "String(own())";
  let result = evaluated_against_a_function_fold(source);

  assert_refused(&result, source);

  assert_eq!(
    result.reason.as_deref(),
    Some("StyleX expression function requires an expression argument."),
    "the sentence must be the argument's own"
  );
}
