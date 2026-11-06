use swc_core::{
  common::comments::Comments,
  ecma::{ast::ComputedPropName, visit::VisitMutWith},
};

use crate::StyleXTransform;

impl<C> StyleXTransform<C>
where
  C: Comments,
{
  pub(crate) fn visit_mut_computed_prop_name_impl(
    &mut self,
    computed_prop_name: &mut ComputedPropName,
  ) {
    computed_prop_name.visit_mut_children_with(self);
  }
}
