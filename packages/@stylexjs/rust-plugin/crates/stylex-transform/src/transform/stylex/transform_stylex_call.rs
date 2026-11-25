use swc_core::{
  common::comments::Comments,
  ecma::ast::{CallExpr, Callee, Expr},
};

use crate::{
  StyleXTransform,
  shared::utils::core::{stylex::stylex, stylex_merge::stylex_merge},
  transform::stylex::transform_stylex_create_call::hoist_expression,
};

impl<C> StyleXTransform<C>
where
  C: Comments,
{
  /// The merge a bare `stylex(...)` call compiles to, or `None` if the call is not one.
  pub(crate) fn transform_stylex_call(&mut self, call: &mut CallExpr) -> Option<Expr> {
    if let Callee::Expr(callee) = &call.callee
      && let Expr::Ident(ident) = callee.as_ref()
      && self.state.matches_namespace(ident)
    {
      return stylex_merge(call, stylex, hoist_expression, false, &mut self.state);
    }

    None
  }
}
