//! A module hoist may move a declaration, never its lexical environment.
use rustc_hash::FxHashSet;
use stylex_state::state_manager::StateManager;
use swc_core::ecma::{
  ast::{
    BindingIdent, ClassDecl, ClassExpr, Expr, FnDecl, FnExpr, Id, Ident, MetaPropExpr, PrivateName,
    Super, ThisExpr, TsEntityName, TsInterfaceDecl, TsTypeAliasDecl, TsTypeParam, TsTypeQuery,
    TsTypeRef,
  },
  visit::{Visit, VisitWith},
};

pub(super) fn can_hoist_to_module(expression: &Expr, state: &StateManager) -> bool {
  let mut owned = OwnedBindings::default();
  expression.visit_with(&mut owned);
  let mut proof = HoistProof {
    state,
    owned: owned.0,
    safe: true,
  };
  expression.visit_with(&mut proof);
  proof.safe
}

#[derive(Default)]
struct OwnedBindings(FxHashSet<Id>);
impl Visit for OwnedBindings {
  fn visit_binding_ident(&mut self, binding: &BindingIdent) {
    self.0.insert(binding.id.to_id());
    binding.visit_children_with(self);
  }
  fn visit_fn_decl(&mut self, function: &FnDecl) {
    self.0.insert(function.ident.to_id());
    function.visit_children_with(self);
  }
  fn visit_fn_expr(&mut self, function: &FnExpr) {
    if let Some(ident) = &function.ident {
      self.0.insert(ident.to_id());
    }
    function.visit_children_with(self);
  }
  fn visit_class_decl(&mut self, class: &ClassDecl) {
    self.0.insert(class.ident.to_id());
    class.visit_children_with(self);
  }
  fn visit_class_expr(&mut self, class: &ClassExpr) {
    if let Some(ident) = &class.ident {
      self.0.insert(ident.to_id());
    }
    class.visit_children_with(self);
  }
  fn visit_ts_type_param(&mut self, parameter: &TsTypeParam) {
    self.0.insert(parameter.name.to_id());
    parameter.visit_children_with(self);
  }
  fn visit_ts_type_alias_decl(&mut self, alias: &TsTypeAliasDecl) {
    self.0.insert(alias.id.to_id());
    alias.visit_children_with(self);
  }
  fn visit_ts_interface_decl(&mut self, interface: &TsInterfaceDecl) {
    self.0.insert(interface.id.to_id());
    interface.visit_children_with(self);
  }
}

struct HoistProof<'a> {
  state: &'a StateManager,
  owned: FxHashSet<Id>,
  safe: bool,
}
impl HoistProof<'_> {
  fn type_name(&mut self, name: &TsEntityName) {
    match name {
      TsEntityName::Ident(ident) => {
        let id = ident.to_id();
        if !self.owned.contains(&id) && !self.state.is_module_root_binding(&id) {
          self.safe = false;
        }
      }
      TsEntityName::TsQualifiedName(name) => self.type_name(&name.left),
    }
  }
}
impl Visit for HoistProof<'_> {
  fn visit_expr(&mut self, expression: &Expr) {
    if self.safe {
      expression.visit_children_with(self);
    }
  }
  fn visit_ident(&mut self, ident: &Ident) {
    let id = ident.to_id();
    if self.owned.contains(&id) || self.state.is_module_root_binding(&id) {
      return;
    }
    if ident.sym == "arguments" || self.state.declared_bindings.contains(&id) {
      self.safe = false;
    }
  }
  fn visit_ts_type_ref(&mut self, reference: &TsTypeRef) {
    self.type_name(&reference.type_name);
    reference.type_params.visit_with(self);
  }
  fn visit_ts_type_query(&mut self, query: &TsTypeQuery) {
    // `typeof` type queries can name runtime bindings from the enclosing scope.
    query.visit_children_with(self);
  }
  fn visit_this_expr(&mut self, _: &ThisExpr) {
    self.safe = false;
  }
  fn visit_super(&mut self, _: &Super) {
    self.safe = false;
  }
  fn visit_private_name(&mut self, _: &PrivateName) {
    self.safe = false;
  }
  fn visit_meta_prop_expr(&mut self, _: &MetaPropExpr) {
    self.safe = false;
  }
}
