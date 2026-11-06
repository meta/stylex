use rustc_hash::FxHashSet;
use swc_core::{
  common::{BytePos, DUMMY_SP, Span},
  ecma::ast::Ident,
};

use crate::state_manager::BindingWrites;
use crate::tests::prelude::{ident, ident_at, test_state as state};

fn span(lo: u32, hi: u32) -> Span {
  Span::new(BytePos(lo), BytePos(hi))
}

fn writes(
  reassigned: &[Ident],
  mutated: &[Ident],
  deeply_mutated: &[Ident],
  declared: &[Ident],
) -> BindingWrites {
  let collect = |idents: &[Ident]| -> FxHashSet<swc_core::ecma::ast::Id> {
    idents.iter().map(Ident::to_id).collect()
  };

  BindingWrites {
    reassignments: collect(reassigned),
    mutations: collect(mutated),
    deep_mutations: collect(deeply_mutated),
    declared: collect(declared),
  }
}

#[test]
fn a_state_that_took_no_pre_scan_knows_of_no_writes() {
  let state = state();
  let subject = ident("styles");

  assert!(!state.has_binding_reassignment(&subject));
  assert!(!state.has_binding_mutation(&subject));
  assert!(!state.has_deep_binding_mutation(&subject));
  assert!(!state.declares_binding(&subject));
}

#[test]
fn each_set_answers_only_its_own_question() {
  let mut state = state();

  state.adopt_binding_writes(writes(
    &[ident("reassigned")],
    &[ident("mutated")],
    &[ident("deeply")],
    &[ident("declared")],
  ));

  assert!(state.has_binding_reassignment(&ident("reassigned")));
  assert!(!state.has_binding_mutation(&ident("reassigned")));

  assert!(state.has_binding_mutation(&ident("mutated")));
  assert!(!state.has_binding_reassignment(&ident("mutated")));

  assert!(state.has_deep_binding_mutation(&ident("deeply")));
  assert!(!state.has_binding_mutation(&ident("deeply")));

  assert!(state.declares_binding(&ident("declared")));
  assert!(!state.has_deep_binding_mutation(&ident("declared")));
}

#[test]
fn a_write_in_one_scope_says_nothing_about_another() {
  let mut state = state();

  state.adopt_binding_writes(writes(
    &[ident_at("styles", 1)],
    &[],
    &[],
    &[ident_at("styles", 1)],
  ));

  assert!(state.has_binding_reassignment(&ident_at("styles", 1)));
  assert!(!state.has_binding_reassignment(&ident_at("styles", 2)));

  assert!(state.declares_binding(&ident_at("styles", 1)));
  assert!(!state.declares_binding(&ident_at("styles", 2)));
}

#[test]
fn a_second_pre_scan_replaces_the_first() {
  let mut state = state();

  state.adopt_binding_writes(writes(&[ident("first")], &[], &[], &[]));
  state.adopt_binding_writes(writes(&[ident("second")], &[], &[], &[]));

  assert!(state.has_binding_reassignment(&ident("second")));
  assert!(!state.has_binding_reassignment(&ident("first")));
}

#[test]
fn a_name_taken_anywhere_is_taken() {
  let mut state = state();

  assert!(!state.has_binding("styles"));

  state.bound_names.insert("styles".to_string());

  assert!(state.has_binding("styles"));
  assert!(!state.has_binding("style"));
}

#[test]
fn a_local_rebinding_blocks_only_the_sites_it_covers() {
  let mut state = state();

  state
    .local_rebinding_scopes
    .insert("stylex".to_string(), vec![span(100, 200)]);

  assert!(state.is_locally_rebound_at("stylex", span(120, 180)));
  assert!(state.is_locally_rebound_at("stylex", span(100, 200)));
  assert!(!state.is_locally_rebound_at("stylex", span(50, 90)));
  assert!(!state.is_locally_rebound_at("stylex", span(150, 250)));
  assert!(!state.is_locally_rebound_at("other", span(120, 180)));
}

#[test]
fn a_name_rebound_twice_blocks_both_places() {
  let mut state = state();

  state
    .local_rebinding_scopes
    .insert("stylex".to_string(), vec![span(10, 20), span(100, 200)]);

  assert!(state.is_locally_rebound_at("stylex", span(12, 18)));
  assert!(state.is_locally_rebound_at("stylex", span(120, 180)));
  assert!(!state.is_locally_rebound_at("stylex", span(50, 60)));
}

#[test]
fn a_name_recorded_with_no_scopes_blocks_nothing() {
  let mut state = state();

  state
    .local_rebinding_scopes
    .insert("stylex".to_string(), vec![]);

  assert!(!state.is_locally_rebound_at("stylex", span(0, 10)));
}

#[test]
fn a_declarator_bound_to_a_pattern_is_recorded_and_indexed_by_nothing() {
  use swc_core::ecma::ast::{Expr, Lit, ObjectPat, Pat, Str, VarDeclarator};

  let mut state = state();

  state.push_declaration(VarDeclarator {
    span: DUMMY_SP,
    name: Pat::Object(ObjectPat {
      span: DUMMY_SP,
      props: vec![],
      optional: false,
      type_ann: None,
    }),
    init: Some(Box::new(Expr::Lit(Lit::Str(Str {
      span: DUMMY_SP,
      value: "styles".into(),
      raw: None,
    })))),
    definite: false,
  });

  assert_eq!(state.declarations().len(), 1);
  assert!(state.declaration_of(&ident("styles")).is_none());
}
