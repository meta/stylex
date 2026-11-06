use std::rc::Rc;

use indexmap::IndexMap;
use stylex_structures::stylex_env::{EnvEntry, JSFunction};

use crate::functions::FunctionConfigType;
use crate::tests::prelude::string_expr;

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
