use swc_core::ecma::ast::Expr;

use crate::resolution::lookup::get_var_decl_by_ident;
use crate::state_manager::StateManager;
use stylex_ast::ast::convertors::create_number_expr;
use stylex_ast::ast::factories::create_ident;

mod get_var_decl_by_ident_tests {}

mod get_var_decl_by_ident_function_map_tests {}

mod get_var_decl_by_ident_fn_map_panic_tests {
  use super::*;
  use crate::functions::{FunctionConfig, FunctionConfigType, FunctionMap, FunctionType};
  use std::rc::Rc;

  #[test]
  #[should_panic(expected = "Function type not supported:")]
  fn panics_for_non_mapper_regular_function() {
    let mut state = StateManager::default();
    let mut fns = FunctionMap::default();
    fn dummy_fn(
      _args: Vec<Expr>,
      _state: &mut StateManager,
      _fns: &crate::functions::FunctionMap,
    ) -> Expr {
      create_number_expr(0.0)
    }
    fns.identifiers.insert(
      "arrFn".into(),
      Box::new(FunctionConfigType::Regular(FunctionConfig {
        fn_ptr: FunctionType::ArrayArgs(dummy_fn),
        takes_path: false,
      })),
    );
    let ident = create_ident("arrFn");
    get_var_decl_by_ident(&ident, &mut state, &fns);
  }

  #[test]
  #[should_panic(expected = "Map values are not supported in this context.")]
  fn panics_for_map_function_config() {
    let mut state = StateManager::default();
    let mut fns = FunctionMap::default();
    fns.identifiers.insert(
      "mapFn".into(),
      Box::new(FunctionConfigType::Map(Default::default())),
    );
    let ident = create_ident("mapFn");
    get_var_decl_by_ident(&ident, &mut state, &fns);
  }

  #[test]
  #[should_panic(expected = "IndexMap values are not supported in this context.")]
  fn panics_for_indexmap_function_config() {
    let mut state = StateManager::default();
    let mut fns = FunctionMap::default();
    fns.identifiers.insert(
      "imapFn".into(),
      Box::new(FunctionConfigType::IndexMap(Rc::new(Default::default()))),
    );
    let ident = create_ident("imapFn");
    get_var_decl_by_ident(&ident, &mut state, &fns);
  }
}

mod get_var_decl_parts_by_ident_tests {}
