//! Tests conversions with no arguments and values that have no object form.

use super::*;

use stylex_ast::ast::convertors::convert_atom_to_string;
use stylex_state::evaluate_result_value::EvaluateResultValue;
use stylex_structures::stylex_options::StyleXOptions;

fn a_path() -> Expr {
  create_string_expr("the call the conversion stands for")
}

/// Runs a conversion with fresh state so refusals do not leak between cases.
#[track_caller]
fn converted(
  conversion: Conversion,
  args: Vec<EvaluateResultValue>,
) -> Option<EvaluateResultValue> {
  let mut state = EvaluationState::new();
  let mut traversal_state = StateManager::new(StyleXOptions::default());

  conversion.of(args, &a_path(), &mut state, &mut traversal_state)
}

#[track_caller]
fn text_of(conversion: Conversion, args: Vec<EvaluateResultValue>, case: &str) -> String {
  match converted(conversion, args) {
    Some(EvaluateResultValue::Expr(Expr::Lit(Lit::Str(text)))) => {
      convert_atom_to_string(&text.value)
    },
    other => panic!(
      "expected `{}` to answer a string, and it answered {:?}",
      case, other
    ),
  }
}

#[track_caller]
fn number_of(conversion: Conversion, args: Vec<EvaluateResultValue>, case: &str) -> f64 {
  match converted(conversion, args) {
    Some(EvaluateResultValue::Expr(Expr::Lit(Lit::Num(number)))) => number.value,
    other => panic!(
      "expected `{}` to answer a number, and it answered {:?}",
      case, other
    ),
  }
}

#[test]
fn the_string_conversion_of_nothing_is_the_empty_string() {
  assert_eq!(text_of(Conversion::String, vec![], "String()"), "");
}

#[test]
fn the_number_conversion_of_nothing_is_zero() {
  assert_eq!(number_of(Conversion::Number, vec![], "Number()"), 0.0);
}

#[test]
fn the_object_conversion_of_nothing_is_an_empty_object() {
  match converted(Conversion::Object, vec![]) {
    Some(EvaluateResultValue::Expr(Expr::Object(object))) => {
      assert!(object.props.is_empty(), "`Object()` carried properties");
    },
    other => panic!(
      "expected `Object()` to answer an object, and it answered {:?}",
      other
    ),
  }
}

#[test]
fn the_object_conversion_refuses_a_value_that_is_not_one() {
  let mut state = EvaluationState::new();
  let mut traversal_state = StateManager::new(StyleXOptions::default());

  let answer = Conversion::Object.of(
    vec![EvaluateResultValue::Null],
    &a_path(),
    &mut state,
    &mut traversal_state,
  );

  assert!(answer.is_none(), "the absent value was converted");

  assert_eq!(
    state.deopt_reason.as_deref(),
    Some(uncoercible_value("Object").as_str()),
    "the refusal must name the global the author wrote"
  );
}
