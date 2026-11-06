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
  /// The merge a bare `stylex(...)` call compiles to, where the call is one.
  ///
  /// One answer for every call this is not. The three the handler wrote out --
  /// a callee that is no expression, an expression that is no name, and a name
  /// that is no `stylex` import -- all mean the same thing to the dispatcher
  /// above, which asks the next handler either way.
  pub(crate) fn transform_stylex_call(&mut self, call: &mut CallExpr) -> Option<Expr> {
    if let Callee::Expr(callee) = &call.callee
      && let Expr::Ident(ident) = callee.as_ref()
      && self.state.is_regular_stylex_import(&ident.sym)
    {
      return stylex_merge(call, stylex, hoist_expression, &mut self.state);
    }

    None
  }
}
