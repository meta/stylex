use swc_core::{
  common::comments::Comments,
  ecma::{ast::Stmt, visit::VisitMutWith},
};

use crate::StyleXTransform;
use stylex_enums::core::TransformationCycle;

impl<C> StyleXTransform<C>
where
  C: Comments,
{
  pub(crate) fn visit_mut_stmts_impl(&mut self, stmts: &mut Vec<Stmt>) {
    if self.state.cycle == TransformationCycle::Finalize {
      stmts.retain(|stmt| {
        // We use `matches` macro as this match is trivial.
        !matches!(stmt, Stmt::Empty(..))
      });
    } else {
      stmts.visit_mut_children_with(self);
    }
  }
}
