//! What the state knows about the bindings a module declares and writes to.
//!
//! Every question here is asked of a *reference*, not of a name: the identifier
//! carries the scope it was resolved in, so a global and a local that spell the
//! same word are two bindings. The one exception is [`StateManager::has_binding`],
//! which is asked of a name on purpose -- it decides whether a name is free to
//! generate, and a name taken in any scope is not free.

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

/// The four sets one pre-scan produced, each holding the bindings named.
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

/// A state that has taken nothing knows of no writes at all.
#[test]
fn a_state_that_took_no_pre_scan_knows_of_no_writes() {
  let state = state();
  let subject = ident("styles");

  assert!(!state.has_binding_reassignment(&subject));
  assert!(!state.has_binding_mutation(&subject));
  assert!(!state.has_deep_binding_mutation(&subject));
  assert!(!state.declares_binding(&subject));
}

/// The four sets are taken together and read back apart: a binding in one of
/// them answers for that question alone.
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

/// The scope is part of the question. A name written in one scope says nothing
/// about the same name in another.
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

/// Taking a second pre-scan replaces the first, rather than adding to it: one
/// walk of the module produces the whole answer.
#[test]
fn a_second_pre_scan_replaces_the_first() {
  let mut state = state();

  state.adopt_binding_writes(writes(&[ident("first")], &[], &[], &[]));
  state.adopt_binding_writes(writes(&[ident("second")], &[], &[], &[]));

  assert!(state.has_binding_reassignment(&ident("second")));
  assert!(!state.has_binding_reassignment(&ident("first")));
}

/// Whether a name is taken anywhere in the module, asked of the name and not of
/// a reference, because a generated name must avoid every scope.
#[test]
fn a_name_taken_anywhere_is_taken() {
  let mut state = state();

  assert!(!state.has_binding("styles"));

  state.bound_names.insert("styles".to_string());

  assert!(state.has_binding("styles"));
  assert!(!state.has_binding("style"));
}

/// A name re-bound locally blocks reuse only where the re-binding covers the
/// site asking. A binding written elsewhere in the module leaves the site free.
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

/// One name re-bound in two places is blocked at either of them.
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

/// A name recorded with no scopes covering it blocks nothing, which is the
/// answer an empty list has to give.
#[test]
fn a_name_recorded_with_no_scopes_blocks_nothing() {
  let mut state = state();

  state
    .local_rebinding_scopes
    .insert("stylex".to_string(), vec![]);

  assert!(!state.is_locally_rebound_at("stylex", span(0, 10)));
}

/// A declarator bound to a pattern declares no single name, so nothing is
/// indexed for it -- but the declarator is still recorded, because its position
/// is what marks the call in it as program level.
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
