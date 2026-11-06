use indexmap::IndexMap;
use std::rc::Rc;
use stylex_constants::constants::{
  api_names::STYLEX_UNSTABLE_CREATE_THEME_NESTED,
  common::VAR_GROUP_HASH_KEY,
  messages::{ONLY_OVERRIDE_DEFINE_VARS_NESTED, non_static_value},
};
use stylex_macros::stylex_panic;
use swc_core::{
  common::comments::Comments,
  ecma::ast::{CallExpr, Expr},
};

use crate::{
  StyleXTransform,
  shared::{
    transformers::stylex_create_theme_nested::stylex_create_theme_nested,
    utils::{
      core::{
        dev_class_name::{convert_theme_to_dev_styles, convert_theme_to_test_styles},
        js_to_ast::convert_values_to_ast,
      },
      validators::{
        argument_at, folded_style_object, or_refuse_nameless_group, validate_define_call,
      },
    },
  },
  transform::stylex::visitor_utils::{build_eval_config, is_call_to},
};
use stylex_diagnostics::code_frame::build_code_frame_error;
use stylex_evaluator::{
  evaluate::{evaluate, evaluate_with_functions},
  evaluate_result::refusal_site,
};
use stylex_state::{
  evaluate_result_value::EvaluateResultValue, functions::FunctionMap, state_manager::ImportKind,
};

impl<C> StyleXTransform<C>
where
  C: Comments,
{
  pub(crate) fn transform_stylex_create_theme_nested_call(
    &mut self,
    call: &CallExpr,
  ) -> Option<Expr> {
    if !is_call_to(
      call,
      &self.state,
      ImportKind::CreateThemeNested,
      STYLEX_UNSTABLE_CREATE_THEME_NESTED,
    ) {
      return None;
    }

    validate_define_call(
      call,
      STYLEX_UNSTABLE_CREATE_THEME_NESTED,
      2,
      &mut self.state,
    );

    let first_arg = argument_at(call, 0, STYLEX_UNSTABLE_CREATE_THEME_NESTED);

    let second_arg = argument_at(call, 1, STYLEX_UNSTABLE_CREATE_THEME_NESTED);

    let evaluated_arg1 = evaluate(first_arg, &mut self.state, &FunctionMap::default());

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
          &non_static_value(STYLEX_UNSTABLE_CREATE_THEME_NESTED),
          &mut self.state,
        )
      )
    };

    validate_nested_theme_variables(&variables);

    let function_map = Rc::new(build_eval_config(&mut self.state));
    let evaluated_arg2 = evaluate_with_functions(second_arg, &mut self.state, function_map);

    // The three answers the second argument can give -- the fold refused, it
    // answered nothing, it answered something that is not an object -- read
    // through the reader the producers of one object share. This handler wrote
    // all three out, with the same two sentences.
    let overrides = folded_style_object(
      evaluated_arg2,
      call,
      second_arg,
      STYLEX_UNSTABLE_CREATE_THEME_NESTED,
      &mut self.state,
    );

    let (mut overrides_obj, injected_styles) = stylex_create_theme_nested(
      &variables,
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

    let injected_styles = self.state.take_nested_rules_before(injected_styles);

    self
      .state
      .register_styles(call, &injected_styles, &result_ast, None);

    Some(result_ast)
  }
}

fn validate_nested_theme_variables(value: &EvaluateResultValue) {
  match value {
    EvaluateResultValue::ThemeRef(theme_ref) => {
      let mut theme_ref = theme_ref.clone();
      let value = theme_ref.get(VAR_GROUP_HASH_KEY);

      // Asked for the refusal alone. `createTheme` reads the same key off the
      // same shape and needs the name; here only the check is wanted, and one
      // reader answers both so the two report the same sentence.
      or_refuse_nameless_group(value.as_css_var());
    },
    _ => validate_nested_theme_variables_object(value),
  }
}

fn validate_nested_theme_variables_object(value: &EvaluateResultValue) {
  let Some(key_values) = value
    .as_expr()
    .and_then(|expr| expr.as_object())
    .map(stylex_ast::ast::convertors::get_key_values_from_object)
  else {
    stylex_panic!("{}", ONLY_OVERRIDE_DEFINE_VARS_NESTED)
  };

  let has_var_group_hash = key_values.into_iter().any(|key_value| {
    stylex_ast::ast::convertors::key_value_name(&key_value) == VAR_GROUP_HASH_KEY
      && key_value
        .value
        .as_lit()
        .and_then(stylex_ast::ast::convertors::convert_lit_to_string)
        .filter(|value| !value.is_empty())
        .is_some()
  });

  if !has_var_group_hash {
    stylex_panic!("{}", ONLY_OVERRIDE_DEFINE_VARS_NESTED);
  }
}
