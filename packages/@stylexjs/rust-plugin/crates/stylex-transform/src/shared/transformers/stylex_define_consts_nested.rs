use indexmap::IndexMap;
use stylex_macros::stylex_panic;
use stylex_nested_config::nested::{
  flatten_nested_consts_config, object_lit_to_nested_consts_config,
};

use crate::shared::{
  transformers::stylex_define_consts::stylex_define_consts,
  utils::core::stylex_nested_utils::{
    UnflattenedCompiledStylesValue, expr_map_to_evaluate_result, unflatten_object,
  },
};
use stylex_constants::constants::messages::VALUES_MUST_BE_OBJECT;
use stylex_state::{
  evaluate_result_value::EvaluateResultValue, state_manager::StateManager,
  types::InjectableStylesMap,
};

pub(crate) fn stylex_define_consts_nested(
  nested_constants: &EvaluateResultValue,
  state: &mut StateManager,
) -> (
  IndexMap<String, UnflattenedCompiledStylesValue>,
  InjectableStylesMap,
) {
  let Some(constants) = nested_constants.as_expr().and_then(|expr| expr.as_object()) else {
    stylex_panic!("{}", VALUES_MUST_BE_OBJECT)
  };

  let nested_constants = object_lit_to_nested_consts_config(constants);
  let flat_constants = flatten_nested_consts_config(&nested_constants);
  let flat_constants = expr_map_to_evaluate_result(flat_constants);
  let (flat_result, injectable_styles) = stylex_define_consts(&flat_constants, state);
  let nested_result = unflatten_object(&flat_result);

  (nested_result, injectable_styles)
}
