//! What each conversion answers for no argument at all, and for a value that is
//! not an object.
//!
//! Asked of the conversion itself rather than through a source, because no
//! source reaches it: `String()` names an unshadowed global with nothing that
//! has to stay on this side, so the fold evaluates it in the engine and the
//! answer never comes back here. The arms are reached only when something else
//! in the call keeps it out of the engine, and with no argument there is no
//! something else.
//!
//! They are kept rather than deleted because each is a different answer from
//! the conversion of `undefined` — `String()` is the empty string where
//! `String(undefined)` is `"undefined"`, and `Number()` is zero where
//! `Number(undefined)` is `NaN` — so an arm that fell through to the argument
//! path would write a value the source does not describe.
//!
//! Measured against the reference implementation for each answer below.

use super::*;

use stylex_ast::ast::convertors::convert_atom_to_string;
use stylex_state::evaluate_result_value::EvaluateResultValue;
use stylex_structures::stylex_options::StyleXOptions;

/// The expression a refusal would be reported against, which no case here reads
/// because none of them refuse on the path the sentence names.
fn a_path() -> Expr {
  create_string_expr("the call the conversion stands for")
}

/// What `conversion` answers for `args`, under a state of its own.
///
/// The state is fresh per case: a conversion writes a refusal onto it, and a
/// case reading the answer of the next one would then read the first one's.
#[track_caller]
fn converted(
  conversion: Conversion,
  args: Vec<EvaluateResultValue>,
) -> Option<EvaluateResultValue> {
  let mut state = EvaluationState::new();
  let mut traversal_state = StateManager::new(StyleXOptions::default());

  conversion.of(args, &a_path(), &mut state, &mut traversal_state)
}

/// The text `conversion` answers for `args`.
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

/// The number `conversion` answers for `args`.
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

/// `String()` is the empty string, which is not what `String(undefined)`
/// answers.
#[test]
fn the_string_conversion_of_nothing_is_the_empty_string() {
  assert_eq!(text_of(Conversion::String, vec![], "String()"), "");
}

/// `Number()` is zero, which is not the `NaN` that `Number(undefined)` answers.
#[test]
fn the_number_conversion_of_nothing_is_zero() {
  assert_eq!(number_of(Conversion::Number, vec![], "Number()"), 0.0);
}

/// `Object()` is a new empty object, which is what the language builds for a
/// call with nothing to convert.
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

/// `Array()` is the empty array, and is the one conversion whose empty answer a
/// source does reach — every argument is an element, so an empty list is
/// already the answer rather than a case of its own.
#[test]
fn the_array_conversion_of_nothing_is_the_empty_array() {
  match converted(Conversion::Array, vec![]) {
    Some(EvaluateResultValue::Vec(items)) => {
      assert!(items.is_empty(), "`Array()` carried elements")
    },
    other => panic!(
      "expected `Array()` to answer an array, and it answered {:?}",
      other
    ),
  }
}

/// A value that stands for nothing at all is refused by the object conversion,
/// naming the global the author wrote.
///
/// Every value the evaluator holds of its own stands for an object or a
/// function, so no module reaches this — the route is the absent value, which
/// the walk records where it read something and found nothing.
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
