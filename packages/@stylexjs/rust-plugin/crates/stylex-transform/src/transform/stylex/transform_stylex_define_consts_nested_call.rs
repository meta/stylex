use rustc_hash::FxHashMap;
use std::rc::Rc;
use stylex_constants::constants::{
  api_names::STYLEX_UNSTABLE_DEFINE_CONSTS_NESTED, messages::cannot_generate_hash,
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
    transformers::stylex_define_consts_nested::stylex_define_consts_nested,
    utils::{
      core::stylex_nested_utils::convert_unflattened_object_to_ast,
      validators::{argument_at, folded_style_object, validate_exported_define_call},
    },
  },
  transform::stylex::visitor_utils::{build_env_only_eval_config, is_call_to},
};
use stylex_evaluator::evaluate::evaluate_with_functions;
use stylex_state::state_manager::ImportKind;

impl<C> StyleXTransform<C>
where
  C: Comments,
{
  pub(crate) fn transform_stylex_define_consts_nested(&mut self, call: &CallExpr) -> Option<Expr> {
    if !is_call_to(
      call,
      &self.state,
      ImportKind::DefineConstsNested,
      STYLEX_UNSTABLE_DEFINE_CONSTS_NESTED,
    ) {
      return None;
    }

    let export_name = validate_exported_define_call(
      call,
      STYLEX_UNSTABLE_DEFINE_CONSTS_NESTED,
      1,
      &mut self.state,
    );

    let first_arg = argument_at(call, 0, STYLEX_UNSTABLE_DEFINE_CONSTS_NESTED);

    let function_map = Rc::new(build_env_only_eval_config(&mut self.state));
    let evaluated_arg = evaluate_with_functions(first_arg, &mut self.state, function_map);

    let value = folded_style_object(
      evaluated_arg,
      call,
      first_arg,
      STYLEX_UNSTABLE_DEFINE_CONSTS_NESTED,
      &mut self.state,
    );

    let file_name = match self
      .state
      .get_filename_for_hashing(&mut FxHashMap::default())
    {
      Some(name) => name,
      None => stylex_panic!(
        "{}",
        cannot_generate_hash(STYLEX_UNSTABLE_DEFINE_CONSTS_NESTED)
      ),
    };

    self.state.export_id = Some(gen_file_based_identifier(&file_name, &export_name, None));

    let (transformed_js_output, js_output) = stylex_define_consts_nested(&value, &mut self.state);
    let result_ast = convert_unflattened_object_to_ast(&transformed_js_output);

    self
      .state
      .register_styles(call, &js_output, &result_ast, None);

    Some(result_ast)
  }
}
