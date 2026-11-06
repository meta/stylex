//! Which function-config shape carries a map.
//!
//! A namespace's API surface is registered as one of four shapes, and only one
//! of them -- the map -- is added to after registration. `as_map_mut` is what
//! the registration asks, so it must decline the other three rather than making
//! a map to hand back.

use std::rc::Rc;

use indexmap::IndexMap;
use stylex_structures::stylex_env::{EnvEntry, JSFunction};
use stylex_utils::collections::FxIndexMap;

use crate::functions::{FunctionConfig, FunctionConfigType, FunctionType};
use crate::tests::prelude::string_expr;

fn regular_config() -> FunctionConfig {
  FunctionConfig {
    fn_ptr: FunctionType::Callback(Box::new(string_expr("callback"))),
    takes_path: false,
  }
}

#[test]
fn a_map_answers_itself_and_can_be_added_to() {
  let mut config = FunctionConfigType::Map(FxIndexMap::default());

  let Some(map) = config.as_map_mut() else {
    panic!("a map config did not answer its map");
  };

  assert!(map.is_empty());
  map.insert("when".into(), FunctionConfigType::Regular(regular_config()));

  // The insert reached the config and not a copy of it, which is the whole
  // reason the accessor hands back a mutable borrow.
  assert_eq!(config.as_map_mut().map(|map| map.len()), Some(1));
}

#[test]
fn a_regular_config_holds_no_map() {
  assert!(
    FunctionConfigType::Regular(regular_config())
      .as_map_mut()
      .is_none()
  );
}

#[test]
fn a_compiled_namespace_holds_no_map() {
  assert!(
    FunctionConfigType::IndexMap(Rc::new(IndexMap::new()))
      .as_map_mut()
      .is_none()
  );
}

#[test]
fn an_env_object_holds_no_map() {
  let mut env = IndexMap::new();

  env.insert("breakpoint".to_string(), EnvEntry::Expr(string_expr("sm")));
  env.insert(
    "spacing".to_string(),
    EnvEntry::Function(JSFunction::new(|_| string_expr("8px"))),
  );

  assert!(
    FunctionConfigType::EnvObject(Rc::new(env))
      .as_map_mut()
      .is_none()
  );
}
