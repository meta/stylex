use std::rc::Rc;

use swc_core::{
  atoms::Atom,
  ecma::ast::{CallExpr, Expr},
};

use crate::shared::{
  transformers::{
    stylex_keyframes::get_keyframes_fn, stylex_position_try::get_position_try_fn,
    stylex_types::get_types_fn,
  },
  utils::validators::is_target_call,
};
use stylex_constants::constants::api_names::{
  STYLEX_ENV, STYLEX_FIRST_THAT_WORKS, STYLEX_KEYFRAMES, STYLEX_POSITION_TRY, STYLEX_TYPES,
  STYLEX_UNSTABLE_CONDITIONAL,
};
use stylex_evaluator::stylex_first_that_works::stylex_first_that_works;
use stylex_state::{
  functions::{FunctionConfig, FunctionConfigType, FunctionMap, FunctionType, RuleCallHelpers},
  state_manager::{ImportKind, StateManager},
  types::{FunctionConfigMap, FunctionMapIdentifiers},
};
use stylex_structures::named_import_source::ImportSources;

pub(crate) fn is_call_to(
  call: &CallExpr,
  state: &StateManager,
  kind: ImportKind,
  name: &str,
) -> bool {
  is_target_call((name, state.get_stylex_api_import(kind)), call, state)
}

/// Register `entry` under `key` on the identifier map a stylex import's name
/// carries, creating the map when that name has none yet.
///
/// Every registration on a namespace import needs the same create-then-insert,
/// and each of the four wrote it by hand -- one of them as an
/// `and_modify`/`or_insert_with` pair that spelled the insert twice.
///
/// A name already bound to something that is not a map keeps it, and the entry
/// is dropped. That is what all four did, and it is what an import spelling an
/// API name over the namespace has always meant here.
pub(crate) fn insert_stylex_identifier_entry(
  identifiers: &mut FunctionMapIdentifiers,
  name: &ImportSources,
  key: Atom,
  entry: FunctionConfigType,
) {
  let identifier = identifiers
    .entry(name.get_import_str().into())
    .or_insert_with(|| Box::new(FunctionConfigType::Map(FunctionConfigMap::default())));

  if let Some(identifier_map) = identifier.as_map_mut() {
    identifier_map.insert(key, entry);
  }
}

/// Registers `env` as an entry of the namespace's own fold, which is the map
/// that answers what properties the namespace *has* — its own keys, a spread
/// of it, and the member read that walks it.
///
/// Beside [`insert_stylex_identifier_entry`] rather than on `StateManager`,
/// because it reads one field of the state and writes the caller's map: living on
/// the state meant `shared::structures` importing out of `transform::stylex` to
/// reach the helper, which is the dependency the other way round from every
/// other pair in this crate.
///
/// Apart from `StateManager::apply_stylex_env`, and called only where the
/// namespace name is a value of the map already: `create`, `defineVars` and
/// `createTheme` each bind the name to a fold, and an `env` the fold does not
/// hold is an object one key short of what the namespace has.
///
/// The calls that leave the name unregistered — `keyframes`, `positionTry`,
/// `viewTransitionClass`, `defineConsts` — are left alone, so a bare `stylex`
/// written where a static value belongs refuses rather than materializing into
/// an object and dropping the declaration silently. Adding the entry there too
/// flipped four of those refusals into silent drops.
///
/// What those calls lose by that is nothing: `stylex.env.<name>` folds in all
/// of them, because the evaluator reads the two-level member off the
/// namespace's member entries — which `apply_stylex_env` registers for every
/// map — rather than off the namespace as a value.
///
/// Registered whether or not the option is set. `env` is a key of the StyleX
/// namespace however the compiler is configured, so an unset option is
/// reported by the member step as the option being unset; an absent entry
/// would be reported as a property nobody can find and send an author looking
/// in their source. It is also what keeps the namespace's key list the same
/// list on every configuration.
pub(crate) fn register_env_in_namespace_fold(state: &StateManager, function_map: &mut FunctionMap) {
  for name in state.stylex_imports() {
    insert_stylex_identifier_entry(
      &mut function_map.identifiers,
      name,
      STYLEX_ENV.into(),
      FunctionConfigType::EnvObject(Rc::clone(&state.options.env)),
    );
  }
}

/// The function map a rule call -- `keyframes`, `positionTry` or
/// `viewTransitionClass` -- evaluates its argument with, built on the first
/// call of the module that asks for `helpers` and kept for the rest.
///
/// One shape written three times before this: each of the three call handlers
/// built the same map for itself, on every call, even in a module that never
/// writes one of the helpers.
///
/// The reference builds the map again for every call, so keeping one for the
/// file rests on two conditions. Both hold, and both are named here because
/// neither is obvious from this function:
///
/// - **Every import is recorded before the first call folds.** The map names
///   the helpers under each local name an import gave them, and `Discover`
///   records every import of the module before `TransformProducers` folds
///   anything, so no name arrives after the map is built. The suite
///   `transform_stylex_keyframes_test::shared_function_map` measures that, with
///   calls standing between the import and the call that reads it.
/// - **`env` is one object for the file.** `build_rule_call_eval_config` folds
///   `apply_stylex_env` into the map, and `CoreStyleXOptions::env` is read-only
///   once the options are built, so a second call cannot be handed a map built
///   from a different env. `the_env_survives_into_every_map_the_module_asks_-
///   for` measures that every map the module asks for still carries it.
///
/// A future `env` that can vary inside one file breaks the second condition,
/// and the answer would be to key the cache on it rather than on `helpers`
/// alone.
pub(crate) fn rule_call_eval_config(
  state: &mut StateManager,
  helpers: RuleCallHelpers,
) -> Rc<FunctionMap> {
  if let Some(function_map) = state.cached_rule_call_function_map(helpers) {
    return Rc::clone(function_map);
  }

  let function_map = Rc::new(build_rule_call_eval_config(state, helpers));

  state.insert_cached_rule_call_function_map(helpers, Rc::clone(&function_map));

  function_map
}

fn build_rule_call_eval_config(state: &StateManager, helpers: RuleCallHelpers) -> FunctionMap {
  let mut function_map = FunctionMap::default();

  register_stylex_helper(
    state,
    &mut function_map,
    ImportKind::FirstThatWorks,
    STYLEX_FIRST_THAT_WORKS,
    &FunctionConfigType::Regular(FunctionConfig {
      fn_ptr: FunctionType::ArrayArgs(stylex_first_that_works),
      takes_path: false,
    }),
  );

  if helpers == RuleCallHelpers::FirstThatWorksAndKeyframes {
    register_stylex_helper(
      state,
      &mut function_map,
      ImportKind::Keyframes,
      STYLEX_KEYFRAMES,
      &FunctionConfigType::Regular(get_keyframes_fn()),
    );
  }

  state.apply_stylex_env(&mut function_map);

  function_map
}

/// Registers one API entry under every local name an import gave it, and as
/// `member_name` on every StyleX namespace the module imports.
///
/// Every function map built in this crate registers most of what it holds this
/// way, and each builder wrote the two loops out again. The entry is a
/// [`FunctionConfigType`] and not a bare [`FunctionConfig`] because the default
/// marker is a compiled object rather than a function, and it is registered the
/// same way.
///
/// The namespace name itself stays unregistered here. Which maps bind it, and
/// why the rest do not, is what [`register_env_in_namespace_fold`] says.
pub(crate) fn register_stylex_helper(
  state: &StateManager,
  function_map: &mut FunctionMap,
  kind: ImportKind,
  member_name: &str,
  entry: &FunctionConfigType,
) {
  register_stylex_identifier(state, function_map, kind, entry);

  for name in state.stylex_imports() {
    function_map
      .member_expressions
      .entry(name.clone())
      .or_default()
      .insert(member_name.into(), Box::new(entry.clone()));
  }
}

/// Registers one API entry under every local name an import gave it, and
/// nowhere else.
///
/// Apart from [`register_stylex_helper`] because not every entry a named import
/// carries is also read off the namespace. `createTheme` reads `positionTry` and
/// `types` by name alone, and registering a member for either would answer a
/// member read the compiler refuses today.
pub(crate) fn register_stylex_identifier(
  state: &StateManager,
  function_map: &mut FunctionMap,
  kind: ImportKind,
  entry: &FunctionConfigType,
) {
  if let Some(set) = state.get_stylex_api_import(kind) {
    for name in set {
      function_map
        .identifiers
        .insert(name.clone(), Box::new(entry.clone()));
    }
  }
}

pub(crate) fn build_eval_config(state: &mut StateManager) -> FunctionMap {
  let mut function_map = FunctionMap::default();

  let types_fn = get_types_fn();

  register_stylex_helper(
    state,
    &mut function_map,
    ImportKind::Keyframes,
    STYLEX_KEYFRAMES,
    &FunctionConfigType::Regular(get_keyframes_fn()),
  );

  register_stylex_helper(
    state,
    &mut function_map,
    ImportKind::PositionTry,
    STYLEX_POSITION_TRY,
    &FunctionConfigType::Regular(get_position_try_fn()),
  );

  register_stylex_identifier(
    state,
    &mut function_map,
    ImportKind::Types,
    &FunctionConfigType::Regular(types_fn.clone()),
  );

  // `types` is the one helper the namespace carries in its own fold rather than
  // as a member alone, because a `create` call reads `stylex.types` off a
  // namespace it also spreads.
  for name in state.stylex_imports() {
    insert_stylex_identifier_entry(
      &mut function_map.identifiers,
      name,
      STYLEX_TYPES.into(),
      FunctionConfigType::Regular(types_fn.clone()),
    );
  }

  apply_unstable_conditional(state, &mut function_map);
  state.apply_stylex_env(&mut function_map);

  // The namespace is a value in this map already -- the fold above binds the
  // name -- so `env` belongs in the fold beside `types`. Without it a bare
  // `stylex` here stands for an object one key short of what it has, and
  // `Object.keys(stylex)` says so.
  register_env_in_namespace_fold(state, &mut function_map);

  function_map
}

pub(crate) fn apply_unstable_conditional(state: &StateManager, function_map: &mut FunctionMap) {
  register_stylex_helper(
    state,
    function_map,
    ImportKind::Conditional,
    STYLEX_UNSTABLE_CONDITIONAL,
    &FunctionConfigType::Regular(get_conditional_fn()),
  );
}

pub(crate) fn build_env_only_eval_config(state: &mut StateManager) -> FunctionMap {
  let mut function_map = FunctionMap {
    disable_imports: true,
    ..FunctionMap::default()
  };

  state.apply_stylex_env(&mut function_map);

  function_map
}

fn get_conditional_fn() -> FunctionConfig {
  FunctionConfig {
    fn_ptr: FunctionType::StylexExprFn(conditional_identity),
    takes_path: false,
  }
}

fn conditional_identity(expr: Expr, _: &mut StateManager) -> Expr {
  expr
}

#[cfg(test)]
#[path = "tests/rule_call_eval_config_test.rs"]
mod rule_call_eval_config_test;
