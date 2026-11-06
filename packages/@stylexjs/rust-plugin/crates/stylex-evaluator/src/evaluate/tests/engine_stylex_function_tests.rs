//! `firstThatWorks` run by the compile-time engine.
//!
//! It is the one StyleX function the engine may call: it answers from its
//! arguments alone and touches no state, so there is nothing to stop the engine
//! running it -- and running it there is what lets a callback fold, since
//! `parts.map((part) => firstThatWorks(part, 'serif'))` is one call per element
//! and that is the engine's job.
//!
//! Which name reaches it is a question about what the module imported rather
//! than about the expression, so every case here records the import a collector
//! would have recorded. Both spellings are covered, because a named import
//! carries the function itself and a namespace import carries an object the
//! call is read off, and the two are separate lookups.
//!
//! What the engine answers has to be what the evaluator's own path answers for
//! the same arguments -- both run the same ordering -- so each case is written
//! as the value the reference implementation writes.

use super::source_evaluation::*;
use crate::stylex_first_that_works::stylex_first_that_works;
use stylex_constants::constants::evaluation_errors::UNDEFINED_CONST;
use stylex_state::{
  functions::{FunctionMap, FunctionType},
  state_manager::{ImportKind, StateManager},
};
use stylex_structures::named_import_source::ImportSources;

/// The name a named import binds the function under.
const IMPORTED: &str = "firstThatWorks";

/// The name a namespace import binds the whole surface under.
const NAMESPACE: &str = "stylex";

/// Records both imports, which is what a module writing either spelling would
/// have left in the state.
fn importing_both(state: &mut StateManager) {
  state.insert_stylex_api_import(ImportKind::FirstThatWorks, IMPORTED.into());
  state.insert_stylex_import(ImportSources::Regular(NAMESPACE.to_string()));
}

/// The function map a `create` call evaluates its argument against, holding
/// `firstThatWorks` under both spellings the imports above bind.
///
/// The optional engine found the function through the imports alone; without
/// it, the function reaches the evaluator the way it reaches the reference
/// implementation -- through the functions map the `create` visitor passes in,
/// as `identifiers.firstThatWorks` and `memberExpressions[ns].firstThatWorks`
/// (`lib/index.js:7215-7245`, built `@stylexjs/babel-plugin` 0.19.1). This is the
/// entry the transform registers (`runtime_function_map.rs`,
/// `FunctionType::ArrayArgs(stylex_first_that_works)`), written out here
/// because the evaluator cannot reach the transform crate.
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

/// The create map with the compiler's own fold beside it, for the cases whose
/// argument is one of the compiler's functions.
fn the_create_function_map_with_a_fold() -> FunctionMap {
  let mut fns = a_function_fold();
  let create = the_create_function_map();

  fns.identifiers.extend(create.identifiers);
  fns.member_expressions.extend(create.member_expressions);

  fns
}

/// What `source` folds to, as text.
///
/// Each expected string is the reference implementation's own for the same
/// source under the same functions map (executed).
#[track_caller]
fn folded(source: &str) -> String {
  folded_text_of(
    evaluated_in_a_state(importing_both, &the_create_function_map(), source),
    source,
  )
}

/// The named spelling and the namespace spelling are the same call and answer
/// the same value.
#[test]
fn both_spellings_of_the_call_answer_the_same_chain() {
  let named = folded(&format!("[{IMPORTED}('var(--accent)', 'blue')].join('')"));
  let through_the_namespace = folded(&format!(
    "[{NAMESPACE}.{IMPORTED}('var(--accent)', 'blue')].join('')"
  ));

  assert_eq!(named, "var(--accent, blue)");
  assert_eq!(named, through_the_namespace);
}

/// A call naming no variable at all has no chain to build, so its arguments are
/// reversed -- highest priority last, as CSS reads a repeated declaration.
#[test]
fn a_call_naming_no_variable_answers_its_arguments_reversed() {
  assert_eq!(
    folded(&format!("{IMPORTED}('a', 'b', 'c').join('-')")),
    "c-b-a"
  );
}

/// A chain with values behind it answers the chain first and then those values,
/// which is the shape the reference implementation answers with.
#[test]
fn a_chain_with_values_behind_it_answers_both() {
  assert_eq!(
    folded(&format!(
      "{IMPORTED}('color-mix(in srgb, currentColor 20%, transparent)', 'var(--accent)', 'blue').join('|')"
    )),
    "var(--accent, blue)|color-mix(in srgb, currentColor 20%, transparent)"
  );
}

/// An argument that is not a string keeps its own form, so a number among the
/// fallbacks reaches CSS the way the language writes it rather than as text the
/// call invented.
#[test]
fn an_argument_that_is_not_a_string_keeps_its_own_form() {
  assert_eq!(folded(&format!("{IMPORTED}(1, 2).join('-')")), "2-1");
  assert_eq!(
    folded(&format!("[{IMPORTED}('var(--size)', 0)].join('')")),
    "var(--size, 0)"
  );
}

/// The call is what a callback runs, which is the whole reason the function
/// travels into the engine: one call per element, none of them this compiler's
/// to make.
#[test]
fn the_call_runs_inside_a_callback() {
  assert_eq!(
    folded(&format!(
      "['var(--a)', 'var(--b)'].map((name) => {IMPORTED}(name, 'serif')).join('|')"
    )),
    "var(--a, serif)|var(--b, serif)"
  );
}

/// A name the module never imported is not this function, and a property of the
/// namespace that is not on the callable surface is not either -- so neither is
/// run by the engine.
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

/// The namespace's property has to be written as a name. A computed property is
/// text the engine would have to resolve before it knew which function was
/// meant, so it names no callable and the call is not run by the engine.
#[test]
fn a_computed_property_of_the_namespace_names_no_callable() {
  let source = format!("[{NAMESPACE}['{IMPORTED}']('var(--a)', 'blue')].join('')");

  assert_refused(
    &evaluated_in_a_state(importing_both, &FunctionMap::default(), &source),
    &source,
  );
}

/// An argument with no compile-time value, inside a callback, fails the
/// callback.
///
/// The callback has run when its body fails, so the failure arrives as a throw
/// rather than a deopt. The reference implementation throws here too: under the
/// same functions map, its `firstThatWorks` is handed the compiler function's
/// config object and throws `TypeError: varName.startsWith is not a function`
/// out of the closure `.map` called (`lib/index.js:6451`, built
/// `@stylexjs/babel-plugin` 0.19.1; executed). This compiler throws at the
/// invocation boundary with its own sentence for an argument that is not a
/// static expression. The refusal sentence this case named was the optional
/// engine's.
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

/// The same argument written *outside* a callback is handed back rather than
/// named, which is the other half of the rule above.
///
/// The dispatch below still owns the call around this one, so a rule raised
/// here would take a fold away from it. What the author reads is the outer
/// call's own refusal.
#[test]
fn an_argument_with_no_value_outside_a_callback_is_handed_back() {
  let source = format!("[{IMPORTED}('var(--a)', {FOLD_FUNCTION})].join('')");

  assert_refused_with(
    &evaluated_in_a_state(importing_both, &a_function_fold(), &source),
    &source,
    UNDEFINED_CONST,
  );
}

/// A name a callback binds is the callback's, whatever the module imported
/// under the same spelling.
///
/// So the callback calls what the parameter holds rather than this function,
/// and a string is not callable. The callback has run when its body fails, so
/// the failure arrives as a throw rather than a deopt. The reference
/// implementation throws here too: the arrow's own identifiers shadow the
/// functions map, its call finds the string `'var(--a)'` as the callee, and
/// `func.apply` throws `TypeError: func.apply is not a function` out of the
/// closure `.map` called (`lib/index.js:6440-6451`, callee read at `:6862-6863`, applied at `:6937`, built
/// `@stylexjs/babel-plugin` 0.19.1; executed). This compiler throws at the
/// invocation boundary with its own sentence for a callee that is not a
/// function. The "not a callable function" wording was the optional engine's.
#[test]
#[should_panic(expected = "Referenced value is not a constant.")]
fn a_callback_parameter_shadowing_the_name_is_the_engines_own_binding() {
  let source = format!("['var(--a)'].map(({IMPORTED}) => {IMPORTED}('x', 'y')).join('')");

  evaluated_in_a_state(importing_both, &the_create_function_map(), &source);
}

/// One StyleX function crosses once, however often the expression calls it. The
/// value under a name is a function of the name, so the second call reads the
/// same object -- and a second parameter of the same name would not print.
///
/// Both spellings are asked, because a named import carries the function itself
/// and a namespace import carries the object the call is read off, which are two
/// crossings rather than one. A function the *module* declared crosses by a
/// third route, which `guarded_walk_tests` reads.
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
