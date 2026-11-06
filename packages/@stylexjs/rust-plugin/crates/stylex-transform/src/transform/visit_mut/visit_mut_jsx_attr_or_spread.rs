use swc_core::{
  common::comments::Comments,
  ecma::{ast::JSXAttrOrSpread, visit::VisitMutWith},
};

use crate::StyleXTransform;
use stylex_enums::core::TransformationCycle;

#[cfg(test)]
#[path = "tests/jsx_call_sites.rs"]
mod current_jsx_call_sites;

impl<C> StyleXTransform<C>
where
  C: Comments,
{
  pub(crate) fn visit_mut_jsx_attr_or_spreads_impl(
    &mut self,
    jsx_attrs: &mut Vec<JSXAttrOrSpread>,
  ) {
    if self.state.cycle == TransformationCycle::Discover {
      for jsx_attr in jsx_attrs.iter() {
        if let JSXAttrOrSpread::SpreadElement(spread) = jsx_attr {
          self.state.seed_jsx_spread_expr(spread.expr.as_ref());
        }
      }
    }

    // Spread replacement happens in `mark_style_vars_to_keep`; here only `Discover` collects
    // spreads.
    jsx_attrs.visit_mut_children_with(self);
  }
}
