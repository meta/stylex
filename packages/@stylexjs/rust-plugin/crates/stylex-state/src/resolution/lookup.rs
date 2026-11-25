//! Which declaration binds a name.

#[cfg(test)]
#[path = "initializer_snapshot_tests.rs"]
mod initializer_snapshot_tests;

use std::rc::Rc;
use stylex_ast::ast::factories::create_var_declarator;
use stylex_macros::{stylex_panic, stylex_unimplemented};
use swc_core::{
  common::Span,
  ecma::ast::{Expr, Ident, VarDeclarator},
};

use crate::{
  functions::{FunctionConfigType, FunctionMap, FunctionType},
  state_manager::StateManager,
};

/// The two parts of a declarator the reference chain actually reads.
pub fn get_var_decl_parts_by_ident(
  ident: &Ident,
  traversal_state: &mut StateManager,
  functions: &FunctionMap,
) -> Option<(Span, Option<Rc<Expr>>)> {
  if let Some(parts) = traversal_state.declaration_parts(ident) {
    return Some(parts);
  }

  get_var_decl_by_ident(ident, traversal_state, functions)
    .map(|declarator| (declarator.span, declarator.init.map(|init| Rc::new(*init))))
}

/// Finds the declarator that an identifier names, or builds one from the function map. # Panics A function-map entry that is a map, an index map, or a regular function that is not a mapper has no declarator to stand for.
pub fn get_var_decl_by_ident(
  ident: &Ident,
  traversal_state: &mut StateManager,
  functions: &FunctionMap,
) -> Option<VarDeclarator> {
  if let Some(var_decl) = traversal_state.declaration_of(ident) {
    return Some(var_decl.clone());
  }

  if let Some(func) = functions.identifiers.get(&ident.sym) {
    match func.as_ref() {
      FunctionConfigType::Regular(func) => match &func.fn_ptr {
        FunctionType::Mapper(func) => {
          let result = func();

          let var_decl = create_var_declarator(ident.clone(), result);

          return Some(var_decl);
        },
        _ => stylex_panic!("Function type not supported: {:?}", func),
      },
      FunctionConfigType::Map(_) => {
        stylex_unimplemented!("Map values are not supported in this context.")
      },
      FunctionConfigType::IndexMap(_) => {
        stylex_unimplemented!("IndexMap values are not supported in this context.")
      },
      FunctionConfigType::EnvObject(_) => return None,
    }
  }

  None
}
