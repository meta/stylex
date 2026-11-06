use swc_core::{
  common::comments::Comments,
  ecma::{ast::Expr, visit::VisitMutWith},
};

use crate::StyleXTransform;
use stylex_ast::ast::convertors::normalize_expr_mut;
use stylex_enums::core::TransformationCycle;

impl<C> StyleXTransform<C>
where
  C: Comments,
{
  pub(crate) fn visit_mut_expr_impl(&mut self, expr: &mut Expr) {
    let normalized_expr = normalize_expr_mut(expr);

    // During Discover, transform compiled JSX/VDOM calls with sx prop and
    // register call expressions for downstream lookup:
    // React/Vue: _jsx("div", { sx: expr }) → _jsx("div", { ...stylex.props(expr) })
    // Solid.js:  _$setAttribute(el, "sx", expr) → _$spread(el, _$mergeProps(() =>
    // stylex.props(expr)), false, true)
    if self.state.cycle == TransformationCycle::Discover {
      if self.transform_sx_in_compiled_jsx(normalized_expr) {
        expr.visit_mut_children_with(self);
        return;
      }
      if let Some(transformed) = self.transform_sx_in_solid_set_attribute(normalized_expr) {
        *expr = transformed;
        expr.visit_mut_children_with(self);
        return;
      }

      if let Some(call_expr) = normalized_expr.as_call() {
        self.state.add_call_expression(call_expr);
      }
    }

    if (self.state.cycle == TransformationCycle::TransformProducers
      || self.state.cycle == TransformationCycle::TransformConsumers)
      && let Some(value) = self.transform_call_expression(normalized_expr)
    {
      *expr = value;
      return;
    }

    expr.visit_mut_children_with(self);
  }
}
