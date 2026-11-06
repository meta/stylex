//! Which declaration binds a name.
//!
//! Two readers over the declarators a module holds, each of which can answer
//! with a declarator the function map synthesizes where the module declares
//! none. The lookups that only read the state's indices are methods on the
//! state itself -- [`StateManager::declaration_of`] and
//! [`StateManager::import_binding`].

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
///
/// Step 5 asks only for the span, which is `Copy`, and step 8 for the
/// initializer. Answering either by cloning the whole declarator also copies its
/// name pattern and type annotation, and does so on the path a real module takes
/// -- the state hit -- where a borrow is available and only the initializer has
/// to be owned.
///
/// Falls through to [`get_var_decl_by_ident`] for the synthesized declarators
/// the function map builds, rather than repeating its arms: that path has to
/// construct one anyway, so there is nothing to save there.
pub fn get_var_decl_parts_by_ident(
  ident: &Ident,
  traversal_state: &mut StateManager,
  functions: &FunctionMap,
) -> Option<(Span, Option<Box<Expr>>)> {
  if let Some(declarator) = traversal_state.declaration_of(ident) {
    return Some((declarator.span, declarator.init.clone()));
  }

  get_var_decl_by_ident(ident, traversal_state, functions)
    .map(|declarator| (declarator.span, declarator.init))
}

/// Finds the declarator that an identifier names, or builds one from the
/// function map.
///
/// # Panics
///
/// A function-map entry that is a map, an index map, or a regular function
/// that is not a mapper has no declarator to stand for. Such an entry is a
/// broken invariant and not a shape that a module can write, so it stops the
/// build with a code frame. An env object answers `None` instead, because an
/// env read is a value that the caller resolves somewhere else.
// The result is owned, so it borrows nothing from the three parameters. The
// parameters therefore need no shared lifetime, and the function above
// declares none either.
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
