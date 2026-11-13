//! Tests member reads from injected function maps and imported variable groups.

use std::rc::Rc;

use super::source_evaluation::*;
use stylex_constants::constants::api_names::FUNCTION_CONFIG_FN_KEY;
use stylex_constants::constants::evaluation_errors::UNEXPECTED_MEMBER_LOOKUP;
use stylex_constants::constants::messages::{EXPECTED_CSS_VAR, MEMBER_NOT_RESOLVED};
use stylex_state::{
  evaluate_result_value::EvaluateResultValue,
  functions::{FunctionConfig, FunctionConfigType, FunctionMap, FunctionType},
  theme_ref::ThemeRef,
};

const GROUP: &str = "colors";

fn map_holding_a_group() -> FunctionMap {
  let theme = ThemeRef::new("vars.stylex.js", "vars", "x");

  let mut fns = FunctionMap::default();

  fns.identifiers.insert(
    GROUP.into(),
    Box::new(FunctionConfigType::Regular(FunctionConfig {
      fn_ptr: FunctionType::ThemeRefMapper(Rc::new(move || theme.clone())),
      takes_path: false,
    })),
  );

  fns
}

#[track_caller]
fn assert_refuses(fns: &FunctionMap, source: &str, reason: &str) {
  assert_refused_with(&evaluated_against(fns, source), source, reason);
}

#[test]
fn a_key_the_namespace_carries_resolves_to_the_entry_under_it() {
  let fns = a_function_fold();

  let expected = match a_folded_function() {
    FunctionConfigType::Regular(config) => EvaluateResultValue::FunctionConfig(config),
    other => panic!("the fold's own entry is a regular config, got {:?}", other),
  };

  for source in [
    format!("{FOLD_NAMESPACE}.{FOLD_ENTRY}"),
    format!("{FOLD_NAMESPACE}['{FOLD_ENTRY}']"),
  ] {
    assert_eq!(
      folded_value_of(evaluated_against(&fns, &source), &source),
      expected,
      "wrong value for `{}`",
      source
    );
  }
}

#[test]
fn a_computed_key_with_no_string_form_refuses() {
  let fns = a_function_fold();

  for source in [
    &format!("{FOLD_NAMESPACE}[['{FOLD_ENTRY}']]"),
    &format!("{FOLD_FUNCTION}[['{FUNCTION_CONFIG_FN_KEY}']]"),
  ] {
    assert_refuses(&fns, source, UNEXPECTED_MEMBER_LOOKUP);
  }
}

#[test]
fn a_member_of_a_group_that_is_not_a_token_refuses() {
  let fns = map_holding_a_group();

  assert_refuses(&fns, &format!("{GROUP}.toString"), EXPECTED_CSS_VAR);
}

#[test]
fn a_group_read_by_a_key_with_no_name_refuses() {
  let fns = map_holding_a_group();

  assert_refuses(&fns, &format!("{GROUP}[true]"), UNEXPECTED_MEMBER_LOOKUP);
  assert_refuses(&fns, &format!("{GROUP}[{{}}]"), MEMBER_NOT_RESOLVED);
}
