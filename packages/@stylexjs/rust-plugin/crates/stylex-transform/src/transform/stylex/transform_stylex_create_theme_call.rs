use indexmap::IndexMap;
use stylex_macros::stylex_panic;
use swc_core::{
  common::comments::Comments,
  ecma::ast::{CallExpr, Expr},
};

use crate::{
  StyleXTransform,
  shared::{
    transformers::{
      stylex_create_theme::stylex_create_theme_from_group, stylex_keyframes::get_keyframes_fn,
      stylex_position_try::get_position_try_fn, stylex_types::get_types_fn,
    },
    utils::{
      core::{
        dev_class_name::{convert_theme_to_dev_styles, convert_theme_to_test_styles},
        js_to_ast::convert_values_to_ast,
      },
      validators::{
        argument_at, is_create_theme_call, validate_stylex_create_theme_indent,
        validate_theme_variables,
      },
    },
  },
  transform::stylex::visitor_utils::{
    apply_unstable_conditional, insert_stylex_identifier_entry, register_env_in_namespace_fold,
    register_stylex_helper, register_stylex_identifier,
  },
};
use stylex_constants::constants::{
  api_names::{STYLEX_CREATE_THEME, STYLEX_KEYFRAMES, STYLEX_TYPES},
  messages::{non_static_value, non_style_object},
};
use stylex_diagnostics::code_frame::build_code_frame_error;
use stylex_evaluator::{evaluate::evaluate, evaluate_result::refusal_site};
use stylex_state::{
  functions::{FunctionConfigType, FunctionMap},
  state_manager::ImportKind,
};

impl<C> StyleXTransform<C>
where
  C: Comments,
{
  pub(crate) fn transform_stylex_create_theme_call(&mut self, call: &CallExpr) -> Option<Expr> {
    let is_create_theme_call = is_create_theme_call(call, &self.state);

    if is_create_theme_call {
      let (_, parent_var_decl) = &self.get_call_var_name(call);

      validate_stylex_create_theme_indent(parent_var_decl, call, &mut self.state);

      let first_arg = argument_at(call, 0, STYLEX_CREATE_THEME);

      let second_arg = argument_at(call, 1, STYLEX_CREATE_THEME);

      let mut function_map = FunctionMap::default();

      let types_fn = FunctionConfigType::Regular(get_types_fn());

      register_stylex_helper(
        &self.state,
        &mut function_map,
        ImportKind::Keyframes,
        STYLEX_KEYFRAMES,
        &FunctionConfigType::Regular(get_keyframes_fn()),
      );

      // `positionTry` and `types` are read by name alone here, so neither is a
      // member of the namespace.
      register_stylex_identifier(
        &self.state,
        &mut function_map,
        ImportKind::PositionTry,
        &FunctionConfigType::Regular(get_position_try_fn()),
      );

      register_stylex_identifier(&self.state, &mut function_map, ImportKind::Types, &types_fn);

      // `types` is carried in the namespace's own fold, because a theme reads
      // `stylex.types` off a namespace it also spreads.
      for name in self.state.stylex_imports() {
        insert_stylex_identifier_entry(
          &mut function_map.identifiers,
          name,
          STYLEX_TYPES.into(),
          types_fn.clone(),
        );
      }

      apply_unstable_conditional(&self.state, &mut function_map);

      self.state.apply_stylex_env(&mut function_map);

      // `env` is carried in the fold for the same reason `types` is: the
      // namespace is a value in this map, so a fold one key short is an object
      // that does not say what the namespace has.
      register_env_in_namespace_fold(&self.state, &mut function_map);

      let function_map: Box<FunctionMap> = Box::new(function_map);

      let evaluated_arg1 = evaluate(first_arg, &mut self.state, &function_map);

      // The fold's two failures are read once. A refusal is the reachable one, and
      // it reads the sentence and the position it always did. A confident answer
      // with no value is the other: the evaluator's memo is its only known
      // producer, and no source through this producer reaches it, so it reads this
      // sentence rather than one of its own. `folded_style_object_lit` reads the
      // two the same way for the producers that share it.
      let Some(variables) = evaluated_arg1.value.filter(|_| evaluated_arg1.confident) else {
        stylex_panic!(
          "{}",
          build_code_frame_error(
            &Expr::Call(call.clone()),
            &refusal_site(evaluated_arg1.deopt.as_ref(), first_arg),
            &non_static_value(STYLEX_CREATE_THEME),
            &mut self.state,
          )
        )
      };

      let evaluated_arg2 = evaluate(second_arg, &mut self.state, &function_map);

      assert!(
        evaluated_arg2.confident,
        "{}",
        build_code_frame_error(
          &Expr::Call(call.clone()),
          &refusal_site(evaluated_arg2.deopt.as_ref(), second_arg),
          &non_static_value(STYLEX_CREATE_THEME),
          &mut self.state,
        )
      );

      // Asked here so that a first argument that is no variable group is
      // refused with this sentence, before the producer runs and reports the
      // same input in its own words. Both arguments are already read at this
      // point, so this does not change which one is read first.
      //
      // The answer is carried on rather than thrown away. Reading it again in
      // the producer copies every property of the group a second time.
      let theme_group = validate_theme_variables(&variables);

      let overrides = match evaluated_arg2.value {
        Some(value) => {
          assert!(
            value
              .as_expr()
              .map(|expr| expr.is_object())
              .unwrap_or(false),
            "{}",
            build_code_frame_error(
              &Expr::Call(call.clone()),
              &refusal_site(evaluated_arg2.deopt.as_ref(), second_arg),
              &non_style_object(STYLEX_CREATE_THEME),
              &mut self.state,
            )
          );
          value
        },
        None => stylex_panic!(
          "{}",
          build_code_frame_error(
            &Expr::Call(call.clone()),
            &refusal_site(evaluated_arg2.deopt.as_ref(), second_arg),
            &non_style_object(STYLEX_CREATE_THEME),
            &mut self.state,
          )
        ),
      };

      let (mut overrides_obj, inject_styles) = stylex_create_theme_from_group(
        theme_group,
        &overrides,
        &mut self.state,
        &mut IndexMap::default(),
      );

      let (var_name, _) = self.get_call_var_name(call);

      if self.state.is_test() {
        overrides_obj =
          convert_theme_to_test_styles(&var_name, &overrides_obj, self.state.get_filename());
      } else if self.state.is_dev() {
        overrides_obj =
          convert_theme_to_dev_styles(&var_name, &overrides_obj, self.state.get_filename());
      }

      let result_ast = convert_values_to_ast(&overrides_obj);

      let injected_styles = self.state.take_nested_rules_before(inject_styles);

      self
        .state
        .register_styles(call, &injected_styles, &result_ast, None);

      Some(result_ast)
    } else {
      None
    }
  }
}
