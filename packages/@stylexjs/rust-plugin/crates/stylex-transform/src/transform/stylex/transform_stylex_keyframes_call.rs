use std::rc::Rc;
use stylex_constants::constants::messages::expected_call_expression;

use indexmap::IndexMap;
use stylex_ast::ast::convertors::create_string_expr;
use stylex_macros::stylex_panic;
use swc_core::{
  common::comments::Comments,
  ecma::ast::{Expr, VarDeclarator},
};

use crate::{
  StyleXTransform,
  shared::{
    transformers::stylex_keyframes::stylex_keyframes,
    utils::validators::{
      argument_at, assert_valid_keyframes, folded_style_object, is_keyframes_call,
      validate_stylex_keyframes_indent,
    },
  },
  transform::stylex::visitor_utils::rule_call_eval_config,
};
use stylex_ast::ast::convertors::init_call;
use stylex_constants::constants::api_names::STYLEX_KEYFRAMES;
use stylex_evaluator::evaluate::evaluate_with_functions;
use stylex_state::functions::RuleCallHelpers;

impl<C> StyleXTransform<C>
where
  C: Comments,
{
  pub(crate) fn transform_stylex_keyframes_call(
    &mut self,
    var_decl: &VarDeclarator,
  ) -> Option<Expr> {
    let is_keyframes_call = is_keyframes_call(var_decl, &self.state);

    if is_keyframes_call {
      validate_stylex_keyframes_indent(var_decl, &mut self.state);

      let call = match init_call(var_decl) {
        Some(call) => call,
        None => stylex_panic!("{}", expected_call_expression(STYLEX_KEYFRAMES)),
      };

      let first_arg = argument_at(call, 0, STYLEX_KEYFRAMES);

      let function_map = rule_call_eval_config(&mut self.state, RuleCallHelpers::FirstThatWorks);

      let evaluated_arg = evaluate_with_functions(first_arg, &mut self.state, function_map);

      let value = folded_style_object(
        evaluated_arg,
        call,
        first_arg,
        STYLEX_KEYFRAMES,
        &mut self.state,
      );

      assert_valid_keyframes(&value, &mut self.state);

      let (animation_name, injectable_style) = stylex_keyframes(&value, &mut self.state);

      let mut injected_styles = IndexMap::new();

      injected_styles.insert(animation_name.clone().into(), Rc::new(injectable_style));

      let result_ast = create_string_expr(animation_name.as_str());

      self
        .state
        .register_styles(call, &injected_styles, &result_ast, None);

      Some(result_ast)
    } else {
      None
    }
  }
}
