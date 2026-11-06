//! Tests `firstThatWorks` imports, callback invocation and shadowing.

use super::source_evaluation::*;
use crate::stylex_first_that_works::stylex_first_that_works;
use stylex_constants::constants::evaluation_errors::UNDEFINED_CONST;
use stylex_state::{
  functions::{FunctionMap, FunctionType},
  state_manager::{ImportKind, StateManager},
};
use stylex_structures::named_import_source::ImportSources;

const IMPORTED: &str = "firstThatWorks";

const NAMESPACE: &str = "stylex";

fn importing_both(state: &mut StateManager) {
  state.insert_stylex_api_import(ImportKind::FirstThatWorks, IMPORTED.into());
  state.insert_stylex_import(ImportSources::Regular(NAMESPACE.to_string()));
}

/// Registers `firstThatWorks` under both import spellings; imports alone do not populate the map.
fn the_create_function_map() -> FunctionMap {
  let entry = folded_entry(FunctionType::ArrayArgs(stylex_first_that_works), false);
  let mut fns = FunctionMap::default();

  fns
    .identifiers
    .insert(IMPORTED.into(), Box::new(entry.clone()));
  fns
    .member_expressions
    .entry(ImportSources::Regular(NAMESPACE.to_string()))
    .or_default()
    .insert(IMPORTED.into(), Box::new(entry));

  fns
}

fn the_create_function_map_with_a_fold() -> FunctionMap {
  let mut fns = a_function_fold();
  let create = the_create_function_map();

  fns.identifiers.extend(create.identifiers);
  fns.member_expressions.extend(create.member_expressions);

  fns
}

#[track_caller]
fn folded(source: &str) -> String {
  folded_text_of(
    evaluated_in_a_state(importing_both, &the_create_function_map(), source),
    source,
  )
}

#[test]
fn both_spellings_of_the_call_answer_the_same_chain() {
  let named = folded(&format!("[{IMPORTED}('var(--accent)', 'blue')].join('')"));
  let through_the_namespace = folded(&format!(
    "[{NAMESPACE}.{IMPORTED}('var(--accent)', 'blue')].join('')"
  ));

  assert_eq!(named, "var(--accent, blue)");
  assert_eq!(named, through_the_namespace);
}

#[test]
fn a_call_naming_no_variable_answers_its_arguments_reversed() {
  assert_eq!(
    folded(&format!("{IMPORTED}('a', 'b', 'c').join('-')")),
    "c-b-a"
  );
}

#[test]
fn a_chain_with_values_behind_it_answers_both() {
  assert_eq!(
    folded(&format!(
      "{IMPORTED}('color-mix(in srgb, currentColor 20%, transparent)', 'var(--accent)', 'blue').join('|')"
    )),
    "var(--accent, blue)|color-mix(in srgb, currentColor 20%, transparent)"
  );
}

#[test]
fn an_argument_that_is_not_a_string_keeps_its_own_form() {
  assert_eq!(folded(&format!("{IMPORTED}(1, 2).join('-')")), "2-1");
  assert_eq!(
    folded(&format!("[{IMPORTED}('var(--size)', 0)].join('')")),
    "var(--size, 0)"
  );
}

#[test]
fn the_call_runs_inside_a_callback() {
  assert_eq!(
    folded(&format!(
      "['var(--a)', 'var(--b)'].map((name) => {IMPORTED}(name, 'serif')).join('|')"
    )),
    "var(--a, serif)|var(--b, serif)"
  );
}

#[test]
fn a_name_the_module_did_not_import_is_not_the_function() {
  for source in [
    String::from("notImported('var(--a)', 'blue')"),
    format!("{NAMESPACE}.notCallable('var(--a)', 'blue')"),
  ] {
    assert_refused(
      &evaluated_in_a_state(importing_both, &FunctionMap::default(), &source),
      &source,
    );
  }
}

#[test]
fn a_computed_property_of_the_namespace_names_no_callable() {
  let source = format!("[{NAMESPACE}['{IMPORTED}']('var(--a)', 'blue')].join('')");

  assert_refused(
    &evaluated_in_a_state(importing_both, &FunctionMap::default(), &source),
    &source,
  );
}

/// Babel throws an unresolved callback-body reference at invocation, rather than returning a deopt.
#[test]
#[should_panic(expected = "Function argument must be a static expression.")]
fn an_argument_with_no_value_refuses_by_the_functions_own_name() {
  let source = format!("['var(--a)'].map((name) => {IMPORTED}(name, {FOLD_FUNCTION})).join('')");

  evaluated_in_a_state(
    importing_both,
    &the_create_function_map_with_a_fold(),
    &source,
  );
}

#[test]
fn an_argument_with_no_value_outside_a_callback_is_handed_back() {
  let source = format!("[{IMPORTED}('var(--a)', {FOLD_FUNCTION})].join('')");

  assert_refused_with(
    &evaluated_in_a_state(importing_both, &a_function_fold(), &source),
    &source,
    UNDEFINED_CONST,
  );
}

#[test]
#[should_panic(expected = "Referenced value is not a constant.")]
fn a_callback_parameter_shadowing_the_name_is_the_engines_own_binding() {
  let source = format!("['var(--a)'].map(({IMPORTED}) => {IMPORTED}('x', 'y')).join('')");

  evaluated_in_a_state(importing_both, &the_create_function_map(), &source);
}

#[test]
fn a_stylex_function_called_twice_crosses_once() {
  assert_eq!(
    folded(&format!(
      "[{IMPORTED}('var(--a)', 'blue'), {IMPORTED}('var(--b)', 'red')].join('|')"
    )),
    "var(--a, blue)|var(--b, red)"
  );
  assert_eq!(
    folded(&format!(
      "[{NAMESPACE}.{IMPORTED}('var(--a)', 'blue'), \
       {NAMESPACE}.{IMPORTED}('var(--b)', 'red')].join('|')"
    )),
    "var(--a, blue)|var(--b, red)"
  );
}
