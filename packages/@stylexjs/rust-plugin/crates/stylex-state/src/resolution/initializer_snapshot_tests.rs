//! The indexed initializer is retained, not deep-cloned for every reference.
use super::get_var_decl_parts_by_ident;
fn object_expr() -> swc_core::ecma::ast::Expr {
  swc_core::ecma::ast::Expr::Object(swc_core::ecma::ast::ObjectLit {
    span: swc_core::common::DUMMY_SP,
    props: Vec::new(),
  })
}
use crate::{functions::FunctionMap, state_manager::StateManager};
use stylex_ast::ast::{factories::create_ident, factories::create_var_declarator};

#[test]
fn initializer_snapshot_reuses_the_same_immutable_ast_handle() {
  let mut state = StateManager::default();
  let id = create_ident("value");
  state.push_declaration(create_var_declarator(id.clone(), object_expr()));
  let first = get_var_decl_parts_by_ident(&id, &mut state, &FunctionMap::default()).unwrap();
  let second = get_var_decl_parts_by_ident(&id, &mut state, &FunctionMap::default()).unwrap();
  assert_eq!(first.0, second.0, "declaration span is TDZ metadata");
  assert!(
    std::ptr::eq(first.1.as_deref().unwrap(), second.1.as_deref().unwrap()),
    "each reference must borrow one retained initializer, not clone its subtree"
  );
}
#[test]
fn retained_initializer_identity_rejects_temporary_and_rewritten_roots() {
  let mut state = StateManager::default();
  let id = create_ident("value");
  state.push_declaration(create_var_declarator(id.clone(), object_expr()));
  let first = get_var_decl_parts_by_ident(&id, &mut state, &FunctionMap::default()).unwrap();
  let owner = first.1.as_deref().unwrap();
  assert!(state.retained_initializer_site(owner).is_some());
  let temporary = owner.clone();
  assert!(state.retained_initializer_site(&temporary).is_none());
  let revision = state.evaluation_input_revision();
  state.set_declaration_init(
    0,
    swc_core::ecma::ast::Expr::Ident(create_ident("replacement")),
  );
  assert_ne!(state.evaluation_input_revision(), revision);
  assert!(state.retained_initializer_site(owner).is_none());
  let second = get_var_decl_parts_by_ident(&id, &mut state, &FunctionMap::default()).unwrap();
  assert!(
    state
      .retained_initializer_site(second.1.as_deref().unwrap())
      .is_some()
  );
}

#[test]
fn initializer_snapshot_changes_after_the_declaration_writer() {
  let mut state = StateManager::default();
  let id = create_ident("value");
  state.push_declaration(create_var_declarator(id.clone(), object_expr()));
  let first = get_var_decl_parts_by_ident(&id, &mut state, &FunctionMap::default()).unwrap();
  state.set_declaration_init(
    0,
    swc_core::ecma::ast::Expr::Ident(create_ident("replacement")),
  );
  let second = get_var_decl_parts_by_ident(&id, &mut state, &FunctionMap::default()).unwrap();
  assert_eq!(
    first.0, second.0,
    "rewriting the value must not rewrite its declaration span"
  );
  assert!(!std::ptr::eq(
    first.1.as_deref().unwrap(),
    second.1.as_deref().unwrap()
  ));
  assert!(
    first.1.as_deref().unwrap().is_object(),
    "an outstanding immutable owner survives the rewrite"
  );
  assert!(second.1.as_deref().unwrap().is_ident());
}
