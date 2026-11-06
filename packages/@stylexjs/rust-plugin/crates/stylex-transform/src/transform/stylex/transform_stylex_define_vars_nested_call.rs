use rustc_hash::FxHashMap;
use std::rc::Rc;
use stylex_constants::constants::{
  api_names::STYLEX_UNSTABLE_DEFINE_VARS_NESTED, messages::cannot_generate_hash,
};
use stylex_macros::stylex_panic;
use stylex_utils::identifier::gen_file_based_identifier;
use swc_core::{
  common::comments::Comments,
  ecma::ast::{CallExpr, Expr},
};

use crate::{
  StyleXTransform,
  shared::{
    transformers::stylex_define_vars_nested::stylex_define_vars_nested,
    utils::{
      core::stylex_nested_utils::convert_unflattened_object_to_ast,
      validators::{argument_at, folded_style_object, validate_exported_define_call},
    },
  },
  transform::stylex::visitor_utils::{build_eval_config, is_call_to},
};
use stylex_evaluator::evaluate::evaluate_with_functions;
use stylex_state::state_manager::ImportKind;

impl<C> StyleXTransform<C>
where
  C: Comments,
{
  pub(crate) fn transform_stylex_define_vars_nested(&mut self, call: &CallExpr) -> Option<Expr> {
    if !is_call_to(
      call,
      &self.state,
      ImportKind::DefineVarsNested,
      STYLEX_UNSTABLE_DEFINE_VARS_NESTED,
    ) {
      return None;
    }

    let export_name =
      validate_exported_define_call(call, STYLEX_UNSTABLE_DEFINE_VARS_NESTED, 1, &mut self.state);

    let first_arg = argument_at(call, 0, STYLEX_UNSTABLE_DEFINE_VARS_NESTED);

    let function_map = Rc::new(build_eval_config(&mut self.state));
    let evaluated_arg = evaluate_with_functions(first_arg, &mut self.state, function_map);

    let value = folded_style_object(
      evaluated_arg,
      call,
      first_arg,
      STYLEX_UNSTABLE_DEFINE_VARS_NESTED,
      &mut self.state,
    );

    let file_name = match self
      .state
      .get_filename_for_hashing(&mut FxHashMap::default())
    {
      Some(name) => name,
      None => stylex_panic!(
        "{}",
        cannot_generate_hash(STYLEX_UNSTABLE_DEFINE_VARS_NESTED)
      ),
    };

    self.state.export_id = Some(gen_file_based_identifier(&file_name, &export_name, None));

    let (variables_obj, injected_styles_sans_keyframes) =
      stylex_define_vars_nested(&value, &mut self.state);

    let injected_styles = self
      .state
      .take_nested_rules_before(injected_styles_sans_keyframes);

    let result_ast = convert_unflattened_object_to_ast(&variables_obj);

    self
      .state
      .register_styles(call, &injected_styles, &result_ast, None);

    Some(result_ast)
  }
}
