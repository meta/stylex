use indexmap::IndexMap;
use stylex_macros::stylex_panic;
use stylex_nested_config::nested::{flatten_nested_vars_config, object_lit_to_nested_vars_config};

use crate::shared::{
  transformers::stylex_define_vars::stylex_define_vars,
  utils::core::stylex_nested_utils::{
    UnflattenedCompiledStylesValue, expr_map_to_evaluate_result, unflatten_object,
  },
};
use stylex_constants::constants::messages::VALUES_MUST_BE_OBJECT;
use stylex_state::{
  evaluate_result_value::EvaluateResultValue, state_manager::StateManager,
  types::InjectableStylesMap,
};

pub(crate) fn stylex_define_vars_nested(
  nested_variables: &EvaluateResultValue,
  state: &mut StateManager,
) -> (
  IndexMap<String, UnflattenedCompiledStylesValue>,
  InjectableStylesMap,
) {
  let Some(variables) = nested_variables.as_expr().and_then(|expr| expr.as_object()) else {
    stylex_panic!("{}", VALUES_MUST_BE_OBJECT)
  };

  let nested_variables = object_lit_to_nested_vars_config(variables);
  let flat_variables = flatten_nested_vars_config(&nested_variables);
  let flat_variables = expr_map_to_evaluate_result(flat_variables);
  let (flat_result, injectable_styles) = stylex_define_vars(&flat_variables, state);

  // The group name carries no separator and is written last, so unflattening
  // leaves it a leaf in the place it was already in.
  (unflatten_object(&flat_result), injectable_styles)
}
