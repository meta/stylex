//! Tests member reads from injected function maps and imported variable groups.

use std::rc::Rc;

use super::source_evaluation::*;
use crate::evaluate::fold_placeholder_function;
use stylex_ast::ast::convertors::create_ident_expr;
use stylex_constants::constants::api_names::FUNCTION_CONFIG_FN_KEY;
use stylex_constants::constants::evaluation_errors::{UNDEFINED_CONST, UNEXPECTED_MEMBER_LOOKUP};
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
fn the_wrapper_key_of_one_entry_resolves_to_a_function() {
  let source = format!("{FOLD_FUNCTION}.{FUNCTION_CONFIG_FN_KEY}");

  assert_eq!(
    folded_value_of(evaluated_against_a_function_fold(&source), &source),
    EvaluateResultValue::Expr(fold_placeholder_function()),
  );
}

#[test]
fn a_key_the_fold_does_not_carry_is_undefined() {
  let fns = a_function_fold();

  for source in [
    &format!("{FOLD_NAMESPACE}.missing"),
    &format!("{FOLD_FUNCTION}.missing"),
  ] {
    let result = evaluated_against(&fns, source);

    assert_eq!(
      folded_value_of(result, source),
      js_undefined_value(),
      "an absent key is undefined"
    );
  }

  assert_eq!(
    folded_text_of(
      evaluated_against(&fns, &format!("{FOLD_NAMESPACE}.missing ?? 'red'")),
      "a fallback beside an absent key"
    ),
    "red"
  );
}

fn js_undefined_value() -> EvaluateResultValue {
  EvaluateResultValue::Expr(create_ident_expr("undefined"))
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
fn a_token_read_off_a_group_resolves_to_its_variable() {
  assert_eq!(read_off_the_group("primary"), "var(--x1ineb92)");
}

#[test]
fn a_nested_token_is_read_by_its_whole_path() {
  assert_eq!(read_off_the_group("brand.primary"), "var(--x1tr9ywo)");
}

#[test]
fn a_token_the_author_named_keeps_its_own_name() {
  let fns = map_holding_a_group();
  let source = format!("{GROUP}['--brand-primary']");

  assert_eq!(
    folded_text_of(evaluated_against(&fns, &source), &source),
    "var(--brand-primary)"
  );
}

#[track_caller]
fn read_off_the_group(path: &str) -> String {
  let fns = map_holding_a_group();
  let source = format!("{GROUP}.{path}");

  folded_text_of(evaluated_against(&fns, &source), &source)
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

#[test]
fn every_member_arm_refuses_a_receiver_that_resolved_to_nothing() {
  for source in [
    "((() => 1) + 1).a",
    "((() => 1) + 1)['a']",
    "((() => 1) + 1)[0]",
    "((() => 1) + 1).length",
  ] {
    assert_refused(&evaluated_after(UNRESOLVED_MEMO_WARM, source), source);
  }
}

#[test]
fn a_computed_key_does_not_give_a_missing_receiver_a_property() {
  let source = "((() => 1) + 1)['a' + 'b']";

  assert_refused(&evaluated_after(UNRESOLVED_MEMO_WARM, source), source);
}

#[test]
fn a_member_read_after_a_refused_argument_changes_no_answer() {
  for source in [
    "[].concat(unknownThing, ({ a: 1 }).a)",
    "[].concat(unknownThing, 'ab'[0])",
    "[].concat(unknownThing, [1, 2][1])",
    "[].concat(unknownThing, [1, 2].length)",
  ] {
    let result = evaluate_source(source);

    assert!(
      !result.confident,
      "`{source}` folded past an argument it had refused"
    );

    assert_eq!(
      result.reason.as_deref(),
      Some(UNDEFINED_CONST),
      "`{source}` was refused by something other than its first argument"
    );
  }
}
