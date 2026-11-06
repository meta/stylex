use std::rc::Rc;

use super::*;
use crate::transform::stylex::visitor_utils::{
  insert_stylex_identifier_entry, register_env_in_namespace_fold, register_stylex_helper,
};

pub(crate) fn build_runtime_function_map<C>(transform: &mut StyleXTransform<C>) -> Rc<FunctionMap>
where
  C: Comments,
{
  let mut function_map = FunctionMap::default();

  // The marker is read here, before the name lists below are borrowed, and is
  // then shared into each registration. One file pays for one build, even where
  // no name registers the marker.
  let marker_values = stylex_default_marker::shared_default_marker_values(&mut transform.state);

  let when_fn = FunctionConfigType::Regular(FunctionConfig {
    fn_ptr: FunctionType::DefaultMarker(Arc::clone(LazyLock::force(&STYLEX_WHEN_MAP))),
    takes_path: false,
  });

  // The four entries a `create` call reads by name and off the namespace alike.
  let helpers = [
    (
      ImportKind::FirstThatWorks,
      STYLEX_FIRST_THAT_WORKS,
      FunctionConfigType::Regular(FunctionConfig {
        fn_ptr: FunctionType::ArrayArgs(stylex_first_that_works),
        takes_path: false,
      }),
    ),
    (
      ImportKind::Keyframes,
      STYLEX_KEYFRAMES,
      FunctionConfigType::Regular(get_keyframes_fn()),
    ),
    (
      ImportKind::PositionTry,
      STYLEX_POSITION_TRY,
      FunctionConfigType::Regular(get_position_try_fn()),
    ),
    (
      ImportKind::DefaultMarker,
      STYLEX_DEFAULT_MARKER,
      FunctionConfigType::IndexMap(Rc::clone(&marker_values)),
    ),
  ];

  for (kind, member_name, entry) in &helpers {
    register_stylex_helper(
      &transform.state,
      &mut function_map,
      *kind,
      member_name,
      entry,
    );
  }

  // `when` is registered by name like the four above, but on the namespace it
  // belongs in the fold rather than beside it: a `create` call spreads the
  // namespace and reads `when` back out of what the spread answered.
  if let Some(set) = transform.state.get_stylex_api_import(ImportKind::When) {
    for name in set {
      function_map
        .identifiers
        .insert(name.clone(), Box::new(when_fn.clone()));
    }
  }

  for name in transform.state.stylex_imports() {
    insert_stylex_identifier_entry(
      &mut function_map.identifiers,
      name,
      STYLEX_WHEN.into(),
      when_fn.clone(),
    );
  }

  transform.state.apply_stylex_env(&mut function_map);

  register_env_in_namespace_fold(&transform.state, &mut function_map);

  // Shared rather than owned. The fold below it asks the evaluator once per
  // style property, and the evaluator has to own the map it hands to every
  // callback it makes -- so a map given by reference was copied at each of
  // those calls. One counted pointer is copied instead.
  Rc::new(function_map)
}
