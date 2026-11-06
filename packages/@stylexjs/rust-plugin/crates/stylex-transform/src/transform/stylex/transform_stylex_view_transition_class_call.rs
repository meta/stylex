use indexmap::IndexMap;
use std::rc::Rc;
use stylex_constants::constants::messages::expected_call_expression;

use stylex_ast::ast::convertors::create_string_expr;
use stylex_macros::stylex_panic;
use swc_core::{
  common::comments::Comments,
  ecma::ast::{Expr, VarDeclarator},
};

use crate::{
  StyleXTransform,
  shared::{
    transformers::stylex_view_transition_class::stylex_view_transition_class,
    utils::validators::{
      argument_at, assert_valid_properties, assert_valid_view_transition_class,
      folded_style_object, is_view_transition_class_call,
      validate_stylex_view_transition_class_indent,
    },
  },
  transform::stylex::visitor_utils::rule_call_eval_config,
};
use stylex_ast::ast::convertors::init_call;
use stylex_constants::constants::{
  api_names::STYLEX_VIEW_TRANSITION_CLASS, common::VALID_VIEW_TRANSITION_CLASS_PROPERTIES,
  messages::VIEW_TRANSITION_CLASS_INVALID_PROPERTY,
};
use stylex_evaluator::evaluate::evaluate_with_functions;
use stylex_state::functions::RuleCallHelpers;

impl<C> StyleXTransform<C>
where
  C: Comments,
{
  pub(crate) fn transform_stylex_view_transition_class_call(
    &mut self,
    var_decl: &VarDeclarator,
  ) -> Option<Expr> {
    let is_view_transition_class_call = is_view_transition_class_call(var_decl, &self.state);

    if is_view_transition_class_call {
      validate_stylex_view_transition_class_indent(var_decl, &mut self.state);

      let call = match init_call(var_decl) {
        Some(call) => call,
        None => stylex_panic!("{}", expected_call_expression(STYLEX_VIEW_TRANSITION_CLASS)),
      };

      let first_arg = argument_at(call, 0, STYLEX_VIEW_TRANSITION_CLASS);

      let function_map =
        rule_call_eval_config(&mut self.state, RuleCallHelpers::FirstThatWorksAndKeyframes);

      let evaluated_arg = evaluate_with_functions(first_arg, &mut self.state, function_map);

      let plain_object = folded_style_object(
        evaluated_arg,
        call,
        first_arg,
        STYLEX_VIEW_TRANSITION_CLASS,
        &mut self.state,
      );

      assert_valid_view_transition_class(&plain_object, &mut self.state);
      assert_valid_properties(
        &plain_object,
        &*VALID_VIEW_TRANSITION_CLASS_PROPERTIES,
        VIEW_TRANSITION_CLASS_INVALID_PROPERTY,
        &mut self.state,
      );

      let (view_transition_class_name, injectable_style) =
        stylex_view_transition_class(&plain_object, &mut self.state);

      let mut own_rules = IndexMap::new();

      own_rules.insert(
        view_transition_class_name.clone().into(),
        Rc::new(injectable_style),
      );

      let injected_styles = self.state.take_nested_rules_before(own_rules);

      let result_ast = create_string_expr(view_transition_class_name.as_str());

      self
        .state
        .register_styles(call, &injected_styles, &result_ast, None);

      Some(result_ast)
    } else {
      None
    }
  }
}
