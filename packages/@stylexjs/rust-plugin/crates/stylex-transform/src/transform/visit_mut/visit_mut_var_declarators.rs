use swc_core::{
  common::comments::Comments,
  ecma::{
    ast::{Pat, VarDeclarator},
    visit::VisitMutWith,
  },
};

use crate::StyleXTransform;
use stylex_enums::core::TransformationCycle;

impl<C> StyleXTransform<C>
where
  C: Comments,
{
  pub(crate) fn visit_mut_var_declarators_impl(
    &mut self,
    var_declarators: &mut Vec<VarDeclarator>,
  ) {
    if self.state.cycle == TransformationCycle::Finalize {
      var_declarators.retain(|decl| {
        if let Pat::Ident(bind_ident) = &decl.name {
          let decl_id = bind_ident.id.to_id();

          // Drop only declarators that the graph captured (`decl_uses`
          // is the observable-set) but whose `Id` is not live. Anything
          // outside the graph — non `Pat::Ident` patterns, helpers
          // introduced after graph capture, etc. — falls through to the
          // "keep by default" fallback.
          if self.state.decl_uses.contains_key(&decl_id) {
            return self.state.live_set.contains(&decl_id);
          }
        }

        true
      });
    }

    var_declarators.visit_mut_children_with(self);
  }
}
