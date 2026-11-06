//! The function map a rule call folds its argument with: that it is memoised per module.
//! The `tests/` suite covers its contents but would still pass without the memo.

use std::rc::Rc;

use indexmap::IndexMap;
use stylex_ast::ast::convertors::create_string_expr;
use stylex_state::{
  functions::{FunctionConfigType, RuleCallHelpers},
  state_manager::{ImportKind, StateManager},
};
use stylex_structures::stylex_env::EnvEntry;
use stylex_structures::{
  named_import_source::ImportSources, stylex_state_options::StyleXStateOptions,
};
use swc_core::atoms::Atom;

use crate::transform::stylex::visitor_utils::rule_call_eval_config;

/// Imports the StyleX namespace and names `firstThatWorks` and `keyframes` through it.
fn state_with_stylex_imports() -> StateManager {
  let mut state = StateManager::for_test(None, StyleXStateOptions::default());

  state.insert_stylex_import(ImportSources::Regular("stylex".to_string()));
  state.insert_stylex_api_import(ImportKind::FirstThatWorks, Atom::from("firstThatWorks"));
  state.insert_stylex_api_import(ImportKind::Keyframes, Atom::from("keyframes"));

  state
}

/// The same object, not an equal one; an equal one would mean the build ran again.
#[test]
fn the_map_is_built_once_for_the_module() {
  let mut state = state_with_stylex_imports();

  let first = rule_call_eval_config(&mut state, RuleCallHelpers::FirstThatWorks);
  let second = rule_call_eval_config(&mut state, RuleCallHelpers::FirstThatWorks);

  assert!(Rc::ptr_eq(&first, &second));
}

/// `viewTransitionClass` folds a `keyframes` call inside a step; the other two must refuse it.
#[test]
fn the_two_helper_sets_answer_different_maps() {
  let mut state = state_with_stylex_imports();

  let fallback_only = rule_call_eval_config(&mut state, RuleCallHelpers::FirstThatWorks);
  let with_keyframes =
    rule_call_eval_config(&mut state, RuleCallHelpers::FirstThatWorksAndKeyframes);

  assert!(!Rc::ptr_eq(&fallback_only, &with_keyframes));

  assert!(
    fallback_only
      .identifiers
      .contains_key(&Atom::from("firstThatWorks"))
  );
  assert!(
    !fallback_only
      .identifiers
      .contains_key(&Atom::from("keyframes"))
  );

  assert!(
    with_keyframes
      .identifiers
      .contains_key(&Atom::from("firstThatWorks"))
  );
  assert!(
    with_keyframes
      .identifiers
      .contains_key(&Atom::from("keyframes"))
  );
}

#[test]
fn one_set_does_not_replace_the_other() {
  let mut state = state_with_stylex_imports();

  let first = rule_call_eval_config(&mut state, RuleCallHelpers::FirstThatWorks);

  rule_call_eval_config(&mut state, RuleCallHelpers::FirstThatWorksAndKeyframes);

  let again = rule_call_eval_config(&mut state, RuleCallHelpers::FirstThatWorks);

  assert!(Rc::ptr_eq(&first, &again));
}

#[test]
fn a_module_with_no_stylex_import_gets_an_empty_map() {
  let mut state = StateManager::for_test(None, StyleXStateOptions::default());

  let function_map = rule_call_eval_config(&mut state, RuleCallHelpers::FirstThatWorks);

  assert!(function_map.identifiers.is_empty());
  assert!(function_map.member_expressions.is_empty());
  assert!(!function_map.disable_imports);
}

/// One map per file relies on `CoreStyleXOptions::env` being read-only once built; an env
/// that varied within a file would have to key the cache.
#[test]
fn the_env_survives_into_every_map_the_module_asks_for() {
  let mut env = IndexMap::new();

  env.insert(
    "brandPrimary".to_string(),
    EnvEntry::Expr(create_string_expr("#123456")),
  );

  let mut state = state_with_stylex_imports();

  state.options.env = Rc::new(env);

  let first = rule_call_eval_config(&mut state, RuleCallHelpers::FirstThatWorks);
  let second = rule_call_eval_config(&mut state, RuleCallHelpers::FirstThatWorksAndKeyframes);

  for map in [&first, &second] {
    let namespace = map
      .member_expressions
      .get(&ImportSources::Regular("stylex".to_string()))
      .expect("the namespace carries the members a rule call may read");

    assert!(
      matches!(
        namespace.get(&Atom::from("env")).map(Box::as_ref),
        Some(FunctionConfigType::EnvObject(_))
      ),
      "the map carries the env object"
    );
  }
}
