use swc_core::{
  common::comments::Comments,
  ecma::{ast::PropName, visit::VisitMutWith},
};

use crate::StyleXTransform;
use stylex_enums::core::TransformationCycle;

impl<C> StyleXTransform<C>
where
  C: Comments,
{
  pub(crate) fn visit_mut_prop_name_impl(&mut self, prop_name: &mut PropName) {
    match self.state.cycle {
      TransformationCycle::Discover if prop_name.is_ident() => {},
      _ => prop_name.visit_mut_children_with(self),
    }
  }
}
