use swc_core::{
  common::comments::Comments,
  ecma::{ast::ExportDefaultExpr, visit::VisitMutWith},
};

use crate::StyleXTransform;
use stylex_ast::ast::convertors::normalize_expr_mut;
use stylex_enums::core::TransformationCycle;

impl<C> StyleXTransform<C>
where
  C: Comments,
{
  pub(crate) fn visit_mut_export_default_expr_impl(
    &mut self,
    export_default_expr: &mut ExportDefaultExpr,
  ) {
    if self.state.cycle == TransformationCycle::Discover {
      export_default_expr.visit_mut_children_with(self);
      return;
    }

    if self.state.cycle == TransformationCycle::TransformProducers
      || self.state.cycle == TransformationCycle::TransformConsumers
    {
      let normalized_expr = normalize_expr_mut(&mut export_default_expr.expr);

      if let Some(value) = self.transform_call_expression(normalized_expr) {
        *export_default_expr.expr = value;

        return;
      }

      // Visit children: the call may be nested, e.g. `export default wrap(stylex.create({ … }))`.
      export_default_expr.visit_mut_children_with(self);
    }
  }
}
