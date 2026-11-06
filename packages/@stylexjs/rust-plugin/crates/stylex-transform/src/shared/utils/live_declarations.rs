//! Which top-level declarations a module still needs: the reference graph, its live set, and
//! the style variables surviving member expressions read. The finalize phase sweeps the rest.

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

/// Collects variable references (by resolver `Id`) in a top-level item, ignoring property keys.
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

/// Builds the top-level reference graph. Simple `Pat::Ident` declarators get edges; every other
/// item's references become roots.
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

/// The transitive closure of `state.roots` over `state.decl_uses`.
/// The sweep also keeps declarators absent from the graph.
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
    // Non-StyleX declarations survive, so their initializer stays an observation point.
    if !state.is_style_var_ident(&bind_ident.id) {
      state.roots.insert(decl_id.clone());
    }
    state
      .decl_uses
      .entry(decl_id)
      .or_default()
      .extend(visitor.idents);
  } else {
    // Untracked (e.g. destructuring) declarators are kept, so their references are roots.
    state.roots.extend(visitor.idents);
  }
}

/// Fills `state.style_vars_to_keep` from surviving style member accesses and applies recorded
/// JSX-spread replacements.
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
            Some(updated_exprs) if !updated_exprs.is_empty() => updated_exprs.to_vec(),
            Some(_) => vec![jsx_attr.clone()],
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
      .is_some_and(|ident| state.matches_namespace(ident))
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

/// The mark step of the finalize phase; the sweep that deletes unused declarations runs after.
pub(crate) fn mark_style_vars_to_keep(module: &mut Module, state: &mut StateManager) {
  let mut visitor = MarkStyleVarsVisitor { state };
  module.visit_mut_with(&mut visitor);
}
