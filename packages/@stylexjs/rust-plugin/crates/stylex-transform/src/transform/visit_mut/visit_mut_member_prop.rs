use swc_core::{
  common::comments::Comments,
  ecma::{ast::MemberProp, visit::VisitMutWith},
};

use crate::StyleXTransform;
use stylex_enums::core::TransformationCycle;

impl<C> StyleXTransform<C>
where
  C: Comments,
{
  pub(crate) fn visit_mut_member_prop_impl(&mut self, member_prop: &mut MemberProp) {
    match self.state.cycle {
      TransformationCycle::Discover if member_prop.is_ident() => {},
      _ => member_prop.visit_mut_children_with(self),
    }
  }
}
