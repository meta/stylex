//! Tests character and entry limits when folded answers are materialized.

use super::source_evaluation::*;
use crate::evaluate_result::EvaluateResult;
use stylex_ast::ast::convertors::convert_atom_to_string;
use stylex_constants::constants::evaluation_errors::{
  folded_string_too_large, object_size_too_large, unsupported_expression,
};
use stylex_state::{evaluate_result_value::EvaluateResultValue, functions::FunctionMap};
use swc_core::ecma::ast::{Expr, Lit, Prop, PropOrSpread};

fn at_a_character_ceiling(limit: usize, source: &str) -> Box<EvaluateResult> {
  evaluated_at_a_character_ceiling(limit, &FunctionMap::default(), source)
}

fn at_an_entry_ceiling(limit: usize, source: &str) -> Box<EvaluateResult> {
  evaluated_at_an_entry_ceiling(limit, &FunctionMap::default(), source)
}

#[track_caller]
fn strings_of(value: EvaluateResultValue, source: &str) -> Vec<String> {
  let value = stylex_evaluator_data(&value, source);
  match value {
    Expr::Array(items) => items
      .elems
      .iter()
      .map(|item| match item {
        Some(item) if item.spread.is_none() => string_of(&item.expr, source),
        _ => panic!("expected `{}` to hold a dense list of strings", source),
      })
      .collect(),
    other => panic!("expected `{}` to fold to an array, got {:?}", source, other),
  }
}

fn stylex_evaluator_data(value: &EvaluateResultValue, source: &str) -> Expr {
  crate::evaluate::materialize_result(value, &stylex_state::state_manager::StateManager::default())
    .expect("the test's evaluated data must materialize without a host error")
    .unwrap_or_else(|| panic!("expected `{source}` to carry materializable data, got {value:?}"))
}

#[track_caller]
fn keys_of(value: EvaluateResultValue, source: &str) -> Vec<String> {
  properties_of(value, source)
    .into_iter()
    .map(|(key, _)| key)
    .collect()
}

#[track_caller]
fn entries_of(value: EvaluateResultValue, source: &str) -> Vec<(String, String)> {
  properties_of(value, source)
    .iter()
    .map(|(key, value)| (key.clone(), string_of(value, source)))
    .collect()
}

#[track_caller]
fn properties_of(value: EvaluateResultValue, source: &str) -> Vec<(String, Expr)> {
  let object = match stylex_evaluator_data(&value, source) {
    Expr::Object(object) => object,
    other => panic!(
      "expected `{}` to fold to an object, got {:?}",
      source, other
    ),
  };

  object
    .props
    .iter()
    .map(|prop| match prop {
      PropOrSpread::Prop(prop) => match prop.as_ref() {
        Prop::KeyValue(pair) => match pair.key.as_ident() {
          Some(name) => (name.sym.to_string(), *pair.value.clone()),
          None => panic!("expected `{}` to fold to identifier keys", source),
        },
        other => panic!(
          "expected `{}` to fold to key-value props, got {:?}",
          source, other
        ),
      },
      PropOrSpread::Spread(_) => {
        panic!("expected `{}` to fold to an object with no spread", source)
      },
    })
    .collect()
}

#[track_caller]
fn string_of(expr: &Expr, source: &str) -> String {
  match expr {
    Expr::Lit(Lit::Str(text)) => convert_atom_to_string(&text.value),
    other => panic!("expected `{}` to hold a string, got {:?}", source, other),
  }
}

#[test]
fn a_string_the_answer_carries_is_counted_against_the_ceiling() {
  let source = "'abcdefgh'.toUpperCase()";

  assert_refused_with(
    &at_a_character_ceiling(4, source),
    source,
    &folded_string_too_large(4),
  );

  assert_eq!(
    folded_text_of(at_a_character_ceiling(8, source), source),
    "ABCDEFGH"
  );
}

#[test]
fn every_string_of_one_answer_is_counted_against_the_same_total() {
  let source = "['abcd', 'efgh'].map((s) => s)";

  assert_refused_with(
    &at_a_character_ceiling(7, source),
    source,
    &folded_string_too_large(7),
  );

  assert_eq!(
    strings_of(
      folded_value_of(at_a_character_ceiling(8, source), source),
      source
    ),
    ["abcd", "efgh"]
  );
}

#[test]
fn a_key_of_a_folded_object_is_counted_as_a_string() {
  let source = "Object.fromEntries([['abcdefgh', 1]])";

  assert_refused_with(
    &at_a_character_ceiling(4, source),
    source,
    &folded_string_too_large(4),
  );

  assert_eq!(
    keys_of(
      folded_value_of(at_a_character_ceiling(8, source), source),
      source
    ),
    ["abcdefgh"]
  );
}

#[test]
fn a_key_and_a_value_share_the_one_total() {
  let source = "Object.fromEntries([['abcd', 'efgh']])";

  assert_refused_with(
    &at_a_character_ceiling(7, source),
    source,
    &folded_string_too_large(7),
  );

  assert_eq!(
    entries_of(
      folded_value_of(at_a_character_ceiling(8, source), source),
      source
    ),
    [("abcd".to_string(), "efgh".to_string())]
  );
}

#[test]
fn an_object_past_the_entry_ceiling_refuses_by_its_property_count() {
  let source = "Object.fromEntries([['a', 1], ['b', 2], ['c', 3]])";

  assert_refused_with(
    &at_an_entry_ceiling(2, source),
    source,
    &object_size_too_large(2),
  );

  assert_eq!(
    keys_of(
      folded_value_of(at_an_entry_ceiling(3, source), source),
      source
    ),
    ["a", "b", "c"]
  );
}

#[test]
fn a_reflective_static_refuses() {
  for method in [
    "create",
    "getPrototypeOf",
    "setPrototypeOf",
    "getOwnPropertyDescriptor",
    "getOwnPropertyDescriptors",
  ] {
    let source = format!("Object.{}({{ a: 1 }}, 'a')", method);
    let result = evaluate_source(&source);

    assert_refused(&result, &source);
    assert_eq!(
      result.reason.as_deref(),
      Some(unsupported_expression("CallExpression").as_str()),
      "the refusal for `{}`",
      source
    );
  }
}

#[test]
fn a_function_under_a_key_is_carried_as_a_function() {
  let source = "Object.fromEntries([['a', () => 1]])";

  assert_folds_to_a_value(source);
  assert_folds_to_string(&format!("Object.keys({source}).join(',')"), "a");
  assert_folds_to_string(&format!("typeof {source}.a"), "function");
}
