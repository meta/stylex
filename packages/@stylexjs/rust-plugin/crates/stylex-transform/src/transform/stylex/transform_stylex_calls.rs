use swc_core::{
  common::comments::Comments,
  ecma::ast::{CallExpr, Expr},
};

use crate::StyleXTransform;
use stylex_enums::core::TransformationCycle;

impl<C> StyleXTransform<C>
where
  C: Comments,
{
  pub(crate) fn transform_stylex_fns(&mut self, call_expr: &mut CallExpr) -> Option<Expr> {
    if self.state.cycle == TransformationCycle::TransformProducers {
      let (_, parent_var_decl) = &self.get_call_var_name(call_expr);

      if let Some(parent_var_decl) = parent_var_decl {
        if let Some(value) = self.transform_stylex_keyframes_call(parent_var_decl) {
          return Some(value);
        }

        if let Some(value) = self.transform_stylex_view_transition_class_call(parent_var_decl) {
          return Some(value);
        }

        if let Some(value) = self.transform_stylex_position_try_call(parent_var_decl) {
          return Some(value);
        }
      }

      if let Some(value) = self.transform_default_marker_call(call_expr) {
        return Some(value);
      }

      if let Some(value) = self.transform_define_marker_call(call_expr) {
        return Some(value);
      }

      if let Some(value) = self.transform_stylex_define_vars(call_expr) {
        return Some(value);
      }

      if let Some(value) = self.transform_stylex_define_vars_nested(call_expr) {
        return Some(value);
      }

      if let Some(value) = self.transform_stylex_define_consts(call_expr) {
        return Some(value);
      }

      if let Some(value) = self.transform_stylex_define_consts_nested(call_expr) {
        return Some(value);
      }

      if let Some(value) = self.transform_stylex_create_theme_call(call_expr) {
        return Some(value);
      }

      if let Some(value) = self.transform_stylex_create_theme_nested_call(call_expr) {
        return Some(value);
      }

      if let Some(value) = self.transform_stylex_create(call_expr) {
        return Some(value);
      }
    }

    if self.state.cycle == TransformationCycle::TransformConsumers {
      if let Some(value) = self.transform_stylex_attrs_call(call_expr) {
        return Some(value);
      }

      if let Some(value) = self.transform_stylex_call(call_expr) {
        return Some(value);
      }

      if let Some(value) = self.transform_stylex_props_call(call_expr) {
        return Some(value);
      }
    }

    None
  }
}
