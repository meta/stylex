use rustc_hash::FxHashMap;
use std::rc::Rc;
use stylex_constants::constants::{
  api_names::STYLEX_DEFINE_CONSTS, messages::cannot_generate_hash,
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
    transformers::stylex_define_consts::stylex_define_consts,
    utils::{
      core::js_to_ast::convert_values_to_ast,
      validators::{
        argument_at, find_and_validate_stylex_define_consts, folded_style_object,
        is_define_consts_call,
      },
    },
  },
  transform::stylex::visitor_utils::build_env_only_eval_config,
};
use stylex_evaluator::evaluate::evaluate_with_functions;

impl<C> StyleXTransform<C>
where
  C: Comments,
{
  pub(crate) fn transform_stylex_define_consts(&mut self, call: &CallExpr) -> Option<Expr> {
    let is_define_consts = is_define_consts_call(call, &self.state);

    if is_define_consts {
      let export_name = find_and_validate_stylex_define_consts(call, &mut self.state);

      let first_arg = argument_at(call, 0, STYLEX_DEFINE_CONSTS);

      let function_map = Rc::new(build_env_only_eval_config(&mut self.state));

      let evaluated_arg = evaluate_with_functions(first_arg, &mut self.state, function_map);

      let value = folded_style_object(
        evaluated_arg,
        call,
        first_arg,
        STYLEX_DEFINE_CONSTS,
        &mut self.state,
      );

      let file_name = match self
        .state
        .get_filename_for_hashing(&mut FxHashMap::default())
      {
        Some(name) => name,
        None => stylex_panic!("{}", cannot_generate_hash(STYLEX_DEFINE_CONSTS)),
      };

      let export_id = Some(gen_file_based_identifier(&file_name, &export_name, None));

      self.state.export_id = export_id;

      let (transformed_js_output, js_output) = stylex_define_consts(&value, &mut self.state);

      let result_ast = convert_values_to_ast(&transformed_js_output);

      self
        .state
        .register_styles(call, &js_output, &result_ast, None);

      Some(result_ast)
    } else {
      None
    }
  }
}
