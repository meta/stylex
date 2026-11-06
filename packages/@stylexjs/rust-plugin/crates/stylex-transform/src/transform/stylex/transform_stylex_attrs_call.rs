use swc_core::{
  common::comments::Comments,
  ecma::ast::{CallExpr, Expr},
};

use crate::{
  StyleXTransform,
  shared::utils::{
    core::{attrs::attrs, stylex_merge::stylex_merge},
    validators::is_attrs_call,
  },
  transform::stylex::transform_stylex_create_call::hoist_expression,
};

impl<C> StyleXTransform<C>
where
  C: Comments,
{
  pub(crate) fn transform_stylex_attrs_call(&mut self, call: &mut CallExpr) -> Option<Expr> {
    let is_attrs_call = is_attrs_call(call, &self.state);

    if is_attrs_call {
      return stylex_merge(call, attrs, hoist_expression, &mut self.state);
    }

    None
  }
}
