use swc_core::{
  common::comments::Comments,
  ecma::{
    ast::{Decl, ExportDecl},
    visit::VisitMutWith,
  },
};

use crate::StyleXTransform;
use stylex_enums::core::TransformationCycle;

impl<C> StyleXTransform<C>
where
  C: Comments,
{
  pub(crate) fn visit_mut_export_decl_impl(&mut self, export_decl: &mut ExportDecl) {
    if self.state.cycle == TransformationCycle::Discover
      && let Decl::Var(var_decl) = &export_decl.decl
    {
      for decl in &var_decl.decls {
        if let Some(ident) = decl.name.as_ident() {
          self.state.roots.insert(ident.id.to_id());
        }
      }
    }

    export_decl.visit_mut_children_with(self);
  }
}
