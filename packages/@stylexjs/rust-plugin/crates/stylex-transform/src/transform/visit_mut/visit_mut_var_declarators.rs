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

          // Babel only deletes remembered StyleX namespace bindings. An
          // unrelated binding stays even when its value was folded or nothing
          // references it; matching uses its full resolved Id, not spelling.
          if self.state.is_style_var_ident(&bind_ident.id)
            && self.state.decl_uses.contains_key(&decl_id)
          {
            return self.state.live_set.contains(&decl_id);
          }
        }

        true
      });
    }

    var_declarators.visit_mut_children_with(self);
  }
}

#[cfg(test)]
#[path = "tests/current_babel_cleanup.rs"]
mod current_babel_cleanup;
