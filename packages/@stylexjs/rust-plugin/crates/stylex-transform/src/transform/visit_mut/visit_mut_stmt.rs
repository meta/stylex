use swc_core::{
  common::{comments::Comments, util::take::Take},
  ecma::{
    ast::{Decl, Stmt},
    utils::IsDirective,
    visit::VisitMutWith,
  },
};

use crate::StyleXTransform;
use stylex_enums::core::TransformationCycle;

impl<C> StyleXTransform<C>
where
  C: Comments,
{
  pub(crate) fn visit_mut_stmt_impl(&mut self, stmt: &mut Stmt) {
    if self.state.cycle == TransformationCycle::Finalize {
      stmt.visit_mut_children_with(self);

      if let Some(Stmt::Decl(Decl::Var(var))) = stmt.as_ref()
        && var.decls.is_empty()
      {
        // Variable declaration without declarator is invalid.
        //
        // After this, `stmt` becomes `Stmt::Empty`.
        stmt.take();
      }
    } else {
      stmt.visit_mut_children_with(self);
    }
  }
}
