use swc_core::{
  common::comments::Comments,
  ecma::ast::{CallExpr, Expr},
};

use std::cell::OnceCell;

use crate::StyleXTransform;
use crate::shared::utils::validators::{
  is_keyframes_call, is_position_try_call, is_view_transition_class_call,
};
use stylex_enums::core::TransformationCycle;
use stylex_utils::hash::stable_hash_unspanned_call;

impl<C> StyleXTransform<C>
where
  C: Comments,
{
  pub(crate) fn transform_stylex_fns(&mut self, call_expr: &mut CallExpr) -> Option<Expr> {
    if self.state.cycle == TransformationCycle::TransformProducers {
      // Producers only read the call, so its key is computed once and shared.
      let call: &CallExpr = call_expr;
      let key = OnceCell::new();
      let call_key = || *key.get_or_init(|| stable_hash_unspanned_call(call));

      // Only these three producers need the declarator, so copy it for them alone.
      let parent_var_decl = self
        .state
        .find_call_declaration_keyed(call, call_key)
        .filter(|decl| {
          is_keyframes_call(decl, &self.state)
            || is_view_transition_class_call(decl, &self.state)
            || is_position_try_call(decl, &self.state)
        })
        .cloned();

      if let Some(parent_var_decl) = &parent_var_decl {
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

      if let Some(value) = self.transform_default_marker_call(call) {
        return Some(value);
      }

      if let Some(value) = self.transform_define_marker_call(call) {
        return Some(value);
      }

      if let Some(value) = self.transform_stylex_define_vars(call) {
        return Some(value);
      }

      if let Some(value) = self.transform_stylex_define_vars_nested(call) {
        return Some(value);
      }

      if let Some(value) = self.transform_stylex_define_consts(call) {
        return Some(value);
      }

      if let Some(value) = self.transform_stylex_define_consts_nested(call) {
        return Some(value);
      }

      if let Some(value) = self.transform_stylex_create_theme_call(call) {
        return Some(value);
      }

      if let Some(value) = self.transform_stylex_create_theme_nested_call(call) {
        return Some(value);
      }

      if let Some(value) = self.transform_stylex_create(call, call_key) {
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
