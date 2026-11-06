//! Which top-level declarations a module still needs.
//!
//! Builds the reference graph between top-level declarations, walks it from the
//! roots to find the set that stays, and marks the style variables the
//! surviving member expressions read. The sweep that deletes the rest runs
//! afterwards, in the finalize phase.

use rustc_hash::FxHashSet;
use stylex_ast::ast::factories::{create_jsx_attr_or_spread, create_jsx_spread_attr};
use stylex_enums::style_vars_to_keep::{NonNullProp, NonNullProps};
use stylex_structures::style_vars_to_keep::StyleVarsToKeep;
use swc_core::ecma::{
  ast::{
    CallExpr, Decl, Expr, Ident, JSXAttrOrSpread, MemberExpr, MemberProp, Module, ModuleDecl,
    ModuleItem, Pat, PropName, Stmt, VarDeclarator,
  },
  visit::{Visit, VisitMut, VisitMutWith, VisitWith},
};

use stylex_ast::ast::keys::namespace_name_from_member_prop;

use crate::shared::utils::validators::{is_attrs_call, is_props_call};
use stylex_state::state_manager::{DeclId, StateManager};

/// Read-only visitor used by [`build_decl_use_graph`] to collect every
/// `Ident` referenced inside a top-level declarator's initializer (or
/// the body of a non-`VarDecl` top-level item).
///
/// Skips identifier-shaped property keys (`{foo: …}`) and member props
/// (`obj.foo`) so that property names do not pollute the reference set —
/// only true variable references are recorded. Each captured ident is
/// stored as its full `Id` (`(Atom, SyntaxContext)`) so resolver-aware
/// shadowing is preserved.
#[derive(Default)]
struct CollectIdentsVisitor {
  idents: FxHashSet<DeclId>,
}

impl Visit for CollectIdentsVisitor {
  fn visit_ident(&mut self, ident: &Ident) {
    self.idents.insert(ident.to_id());
  }

  fn visit_member_prop(&mut self, member_prop: &MemberProp) {
    if !member_prop.is_ident() {
      member_prop.visit_children_with(self);
    }
  }

  fn visit_prop_name(&mut self, prop_name: &PropName) {
    if !prop_name.is_ident() {
      prop_name.visit_children_with(self);
    }
  }
}

/// Build the reference graph used by the new cleanup pass.
///
/// Walks `module.body` once. Top-level `VarDeclarator`s with a simple
/// `Pat::Ident` binding contribute an edge from their `DeclId` to every
/// `DeclId` referenced in their initializer. Top-level items that are
/// not declarators (function decls, class decls, expression statements,
/// non-`VarDecl` exports) are treated as observation points: every
/// `DeclId` they reference is added to `state.roots` directly.
///
/// The graph is consumed by [`compute_live_set`] to compute reachability
/// from `roots` and decide which declarators survive the sweep.
pub(crate) fn build_decl_use_graph(module: &Module, state: &mut StateManager) {
  for item in &module.body {
    match item {
      ModuleItem::Stmt(Stmt::Decl(Decl::Var(var_decl))) => {
        for decl in &var_decl.decls {
          collect_decl_uses(state, decl);
        }
      },
      ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export_decl)) => match &export_decl.decl {
        Decl::Var(var_decl) => {
          for decl in &var_decl.decls {
            collect_decl_uses(state, decl);
          }
        },
        other_decl => {
          let mut visitor = CollectIdentsVisitor::default();
          other_decl.visit_with(&mut visitor);
          state.roots.extend(visitor.idents);
        },
      },
      _ => {
        let mut visitor = CollectIdentsVisitor::default();
        item.visit_with(&mut visitor);
        state.roots.extend(visitor.idents);
      },
    }
  }
}

/// Compute the transitive closure of `state.roots` over `state.decl_uses`.
///
/// Returns the set of `DeclId`s that are reachable from any root via the
/// reference graph. The sweep keeps every declarator whose `DeclId` is
/// either in the returned set or absent from `state.decl_uses` entirely
/// (the "not-in-graph ⇒ keep by default" fallback).
///
/// Iterative breadth-first traversal with a worklist; cycles and
/// self-references terminate naturally because already-marked nodes are
/// not revisited.
pub(crate) fn compute_live_set(state: &StateManager) -> FxHashSet<DeclId> {
  let mut live: FxHashSet<DeclId> = FxHashSet::default();
  let mut worklist: Vec<DeclId> = state.roots.iter().cloned().collect();

  while let Some(node) = worklist.pop() {
    if !live.insert(node.clone()) {
      continue;
    }
    if let Some(targets) = state.decl_uses.get(&node) {
      for target in targets {
        worklist.push(target.clone());
      }
    }
  }

  live
}

fn collect_decl_uses(state: &mut StateManager, decl: &VarDeclarator) {
  let mut visitor = CollectIdentsVisitor::default();
  if let Some(init) = &decl.init {
    init.visit_with(&mut visitor);
  }

  if let Pat::Ident(bind_ident) = &decl.name {
    let decl_id: DeclId = bind_ident.id.to_id();
    state
      .decl_uses
      .entry(decl_id)
      .or_default()
      .extend(visitor.idents);
  } else {
    // Non-`Pat::Ident` declarators (destructuring, etc.) are not tracked
    // by the graph; they fall through to the sweep's "absent ⇒ keep"
    // fallback. Treat their referenced idents as roots so anything they
    // depend on is preserved.
    state.roots.extend(visitor.idents);
  }
}

/// Visitor used by [`mark_style_vars_to_keep`] to populate
/// `state.style_vars_to_keep` from the surviving member-expression accesses
/// on style namespaces and to materialize any JSX-spread replacements
/// recorded during the discovery phase.
///
/// Replaces what the legacy `TransformationCycle::PreCleaning` arms in
/// `visit_mut_member_expr.rs` and `visit_mut_jsx_attr_or_spread.rs` did,
/// so the finalize phase can collapse to a single sweep cycle.
struct MarkStyleVarsVisitor<'a> {
  state: &'a mut StateManager,
}

impl VisitMut for MarkStyleVarsVisitor<'_> {
  fn visit_mut_call_expr(&mut self, call: &mut CallExpr) {
    if is_stylex_consumer_call(call, self.state) {
      return;
    }

    call.visit_mut_children_with(self);
  }

  fn visit_mut_member_expr(&mut self, member_expression: &mut MemberExpr) {
    if let Expr::Ident(ident) = member_expression.obj.as_ref()
      && self.state.is_style_var_ident(ident)
      && let Some(namespace_name) = member_namespace_name(&member_expression.prop)
    {
      let decl_id = ident.to_id();
      self.state.roots.insert(decl_id.clone());
      self.state.style_vars_to_keep.insert(StyleVarsToKeep(
        decl_id,
        namespace_name,
        NonNullProps::True,
      ));
    }

    member_expression.visit_mut_children_with(self);
  }

  fn visit_mut_jsx_attr_or_spreads(&mut self, jsx_attrs: &mut Vec<JSXAttrOrSpread>) {
    let mut result: Vec<JSXAttrOrSpread> = jsx_attrs
      .iter()
      .flat_map(|jsx_attr| match jsx_attr {
        JSXAttrOrSpread::SpreadElement(spread) => {
          match self.state.jsx_spread_replacement(spread.expr.as_ref()) {
            // A recorded spread with replacement attrs — substitute them.
            Some(updated_exprs) if !updated_exprs.is_empty() => updated_exprs.to_vec(),
            // A recorded spread with no replacement attrs — keep the original.
            Some(_) => vec![jsx_attr.clone()],
            // Not a recorded spread — keep the original spread element.
            None => vec![create_jsx_spread_attr(*spread.expr.clone())],
          }
        },
        JSXAttrOrSpread::JSXAttr(attr) => vec![create_jsx_attr_or_spread(attr.clone())],
      })
      .collect();

    result.visit_mut_children_with(self);
    *jsx_attrs = result;
  }
}

fn is_stylex_consumer_call(call: &CallExpr, state: &StateManager) -> bool {
  is_props_call(call, state)
    || is_attrs_call(call, state)
    || call
      .callee
      .as_expr()
      .and_then(|callee| callee.as_ident())
      .is_some_and(|ident| state.is_regular_stylex_import(&ident.sym))
}

fn member_namespace_name(member_prop: &MemberProp) -> Option<NonNullProp> {
  if let Some(namespace_name) = namespace_name_from_member_prop(member_prop) {
    Some(NonNullProp::Atom(namespace_name))
  } else if member_prop.is_computed() {
    Some(NonNullProp::True)
  } else {
    None
  }
}

/// Walk the module to populate `state.style_vars_to_keep` from surviving
/// member-expr accesses on style namespaces, and to apply any deferred
/// JSX-spread replacements collected during discovery.
///
/// This is the "mark" step of the finalize phase. The "sweep" step that
/// actually deletes unused declarations runs afterwards under
/// `TransformationCycle::Finalize`.
pub(crate) fn mark_style_vars_to_keep(module: &mut Module, state: &mut StateManager) {
  let mut visitor = MarkStyleVarsVisitor { state };
  module.visit_mut_with(&mut visitor);
}
