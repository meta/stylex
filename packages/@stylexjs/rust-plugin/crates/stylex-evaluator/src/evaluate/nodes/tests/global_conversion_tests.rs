//! `String`, `Number`, `Object` and `Array` over a value the engine never sees.
//!
//! All four are native JavaScript functions and fold by being called, which is
//! what the [applied global suite](super::applied_global_tests) asserts. What
//! is left for this compiler to answer is the argument the bridge cannot carry:
//! the folded function map is not a JavaScript value at all, so a call over one
//! is handed back unfolded and the conversion is applied here instead.
//!
//! Upstream folds every one of these, so a build that refused what upstream
//! converts would name a class the other build defines. The two coercions that
//! do refuse -- a value with no string form and one with no numeric form --
//! refuse in both compilers, and the sentence has to name the global the author
//! wrote rather than the shape this compiler holds.

use crate::evaluate::source_evaluation::*;
use stylex_ast::ast::convertors::{
  convert_atom_to_string, convert_key_value_to_str, create_number_expr,
};
use stylex_constants::constants::evaluation_errors::{
  NUMERIC_CONVERSION, SPREAD_ELEMENT, STRING_CONVERSION, unbounded_declared_length,
  uncoercible_value,
};
use stylex_state::evaluate_result_value::EvaluateResultValue;
use swc_core::ecma::ast::Expr;

/// The refusal a conversion gives, so a case can say which global it names.
#[track_caller]
fn assert_refuses_naming(source: &str, global: &str) {
  assert_refused_with(
    &evaluated_against_a_function_fold(source),
    source,
    &uncoercible_value(global),
  );
}

/// The value a conversion answered, for the two that answer rather than refuse.
#[track_caller]
fn folded_value(source: &str) -> EvaluateResultValue {
  folded_value_of(evaluated_against_a_function_fold(source), source)
}

/// Asserts a conversion over the fold answers `NaN`.
#[track_caller]
fn assert_conversion_is_nan(source: &str) {
  assert_result_folds_to_nan(evaluated_against_a_function_fold(source), source);
}

/// A single function of the fold has no string form -- its only string would be
/// its source text, and this compiler keeps none -- so the string conversion
/// refuses. The sentence names `String`, which is what the author wrote, and
/// not the shape this compiler was holding.
#[test]
fn the_string_conversion_refuses_a_value_with_no_string_form() {
  assert_refuses_naming("String(own)", "String");
}

/// The numeric conversion answers rather than refuses: `Number` of a function
/// is `NaN` in the language, and `NaN` flows into the declaration exactly as it
/// does upstream.
#[test]
fn the_number_conversion_answers_not_a_number_for_a_function() {
  assert_conversion_is_nan("Number(own)");
}

/// The namespace is an object where one of its functions is a function, and
/// that is what both coercions turn on: `import * as stylex` binds an object
/// whose properties happen to be functions, so the object default answers where
/// the function refuses. The reference implementation reads it the same way.
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

/// `Object` of an object is that object, so the fold is handed straight back --
/// which is what keeps a member read off the result resolving to the same thing
/// the bare name resolves to.
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

/// `Array` has no surplus argument: every argument is an element, so the whole
/// list is read and a call over one value answers a one-element array.
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

/// `String`, `Number` and `Object` read the first argument and ignore the rest,
/// as they do in the language: the surplus changes neither the answer nor the
/// refusal.
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

/// A global that is not one of the four names no conversion, so the call falls
/// through to the dispatch below rather than being converted -- `Math` is not a
/// function at all.
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

/// `Array` of a lone number is a declared length rather than an element, and
/// this compiler refuses it: the number reaching here was never measured
/// against the fold's own ceiling, so building the array would allocate on a
/// count nothing bounded. The usual spelling is folded by the engine, which
/// applies that ceiling; one reaches here only when the rest of the call
/// declined.
#[test]
fn the_array_conversion_refuses_a_length_it_cannot_bound() {
  let source = "Array(String(sx).length)";

  assert_refused_with(
    &evaluated_against_a_function_fold(source),
    source,
    &unbounded_declared_length("Array"),
  );
}

/// The numeric conversion refuses a value with no number at all, and names the
/// global the author wrote. `{ toString: 1 }` is such a value: neither
/// conversion method is callable, so the language throws where a number was
/// wanted rather than answering one.
#[test]
fn the_number_conversion_refuses_a_value_with_no_numeric_form() {
  assert_refuses_naming("Number({ toString: 1 }, own)", "Number");
}

/// The string conversion grows its answer against the character ceiling, so an
/// argument whose text passes it refuses with the ceiling's own sentence rather
/// than with the uncoercible one.
#[test]
fn the_string_conversion_refuses_a_text_past_the_ceiling() {
  assert_refused_at_the_character_ceiling(
    4,
    &a_function_fold(),
    STRING_CONVERSION,
    "String(['1234567890'], own)",
  );
}

/// The numeric conversion reads its number off a text, and that text is
/// measured against the character ceiling as it grows -- so an argument past it
/// refuses with the ceiling's own sentence rather than with the uncoercible
/// one.
#[test]
fn the_number_conversion_refuses_a_text_past_the_ceiling() {
  assert_refused_at_the_character_ceiling(
    4,
    &a_function_fold(),
    NUMERIC_CONVERSION,
    "Number(['1234567890'], own)",
  );
}

/// The two conversions that read an ordinary value answer it the way the
/// language does, on the path this compiler owns as well as in the engine: a
/// number is its own number, and `Object` of an object is that object.
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

/// A spread argument refuses before any conversion is reached, and the sentence
/// is the spread's rather than the conversion's.
///
/// One written element stands for however many the spread holds, so the list a
/// conversion would read is not the list the author wrote. The guard walks the
/// arguments of a call it owns before the conversion is applied, and a
/// conversion name is a global it owns — so all four names refuse identically,
/// at the same step, for a reason that is not the conversion's at all.
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

/// An argument that refuses while it is being evaluated leaves the list shorter
/// than the author wrote, and the conversion reaches its own refusal for the
/// shifted list rather than converting one that no longer lines up.
///
/// The sentence stays the argument's: the argument refused first, so the
/// conversion's own refusal is a no-op over it. That is the order worth
/// pinning — naming `String` here would report the brackets for a mistake
/// inside them.
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
