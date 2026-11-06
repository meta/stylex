use std::{borrow::Borrow, rc::Rc, sync::Arc};
use swc_core::common::Spanned;

mod binding;
mod builtin_value;
mod cache;
mod enumerable_values;
mod iterable_values;
mod member_observer;
mod primitive;
mod text_boundary;
mod truthiness;
pub use member_observer::{MemberReadObserver, with_member_read_observer};
mod callback_failure;
mod deopt;
#[path = "engine_free.rs"]
mod engine_fold;
mod function_scope;
mod helpers;
pub use function_scope::{evaluate_function_in_scope, evaluate_function_in_scope_observed};
mod shallow_boundary;
pub use shallow_boundary::materialize_result_shallow;
mod heap_values;
mod host_chain;
mod host_dispatch;
mod host_effects;
mod host_operators;
mod host_values;
pub use host_values::{host_primitive_expr, materialize_result};
mod native_call;
mod nodes;
mod rebuild;

pub(crate) use cache::{Memoized, evaluate_cached, folded_once};
pub(crate) use deopt::{deopt, deopt_at_declaration};
pub use engine_fold::prints_a_check;
pub use helpers::evaluate_result_is_nullish;
use helpers::*;
pub(crate) use nodes::binary_expression::binary_expr_to_num_or_str;
pub use nodes::object_expression::spread_own_properties;

use rebuild::resolve_env_entry_to_result;
pub(crate) use rebuild::{array_element_expr, evaluate_result_as_expr, fold_placeholder_function};
pub use rebuild::{evaluate_result_vec_to_array_expr, function_fold_to_object};

use indexmap::IndexMap;
use log::{debug, warn};
use rustc_hash::FxHashMap;
use stylex_macros::deopt_unsupported;
use swc_core::{
  atoms::Atom,
  ecma::{
    ast::{
      ArrayLit, ArrowFunctionBody, CallExpr, Callee, ComputedPropName, Expr, ExprOrSpread, Ident,
      ImportSpecifier, KeyValueProp, Lit, MemberProp, ModuleExportName, ObjectLit, OptChainBase,
      Pat, Prop, PropName, PropOrSpread, TplElement,
    },
    utils::ident::IdentLike,
  },
};

use crate::convertors::expr_to_num;
use crate::{evaluate_result::EvaluateResult, state::EvaluationState};
#[cfg(test)]
use stylex_ast::ast::convertors::convert_atom_to_string;
use stylex_ast::ast::convertors::{
  create_big_int_expr, create_bool_expr, create_number_expr, create_string_expr, is_js_undefined,
  key_value_name, normalize_expr,
};
use stylex_ast::ast::factories::{
  create_array_expression, create_expr_or_spread, create_ident_key_value_prop, create_object_lit,
};
use stylex_ast::ast::objects::{assign_props, order_own_keys, remove_duplicates};
use stylex_constants::constants::{
  evaluation_errors::{
    CONCATENATION, IMPORT_FILE_EVAL_ERROR, IMPORT_PATH_RESOLUTION_ERROR, NON_CONSTANT,
    NUMERIC_CONVERSION, OBJECT_METHOD, PATH_WITHOUT_NODE, SPREAD_ELEMENT, UNEXPECTED_MEMBER_LOOKUP,
    USED_BEFORE_DECLARATION, grown_string_too_large, unfoldable_call, unsupported_expression,
    unsupported_operator,
  },
  messages::{
    ARGUMENT_NOT_EXPRESSION, EXPECTED_CSS_VAR, EXPRESSION_IS_NOT_A_STRING,
    ILLEGAL_PROP_ARRAY_VALUE, ILLEGAL_PROP_VALUE, KEY_HAS_NO_NAME, MEMBER_NOT_RESOLVED,
    NULLISH_TO_OBJECT, OBJECT_KEY_MUST_BE_IDENT, PROPERTY_NOT_FOUND, SPREAD_PROPERTIES_UNREADABLE,
    THEME_IMPORT_KEY_AS_OBJECT_KEY, VALUE_MUST_BE_LITERAL,
  },
};
use stylex_enums::{
  import_path_resolution::ImportPathResolution, misc::BinaryExprType,
  value_with_default::ValueWithDefault,
};
use stylex_js::coercions;
use stylex_js::coercions::{global_identifier_to_value, is_global_spelled_as_an_identifier};
use stylex_js::helpers::{
  get_callee_name, get_method_name, is_a_valid_callee_name, is_id_prop, is_mutating_object_method,
  is_mutation_expr, is_valid_callee_method,
};
use stylex_state::resolution::lookup::get_var_decl_parts_by_ident;
use stylex_state::{
  evaluate_result_value::EvaluateResultValue,
  functions::{FunctionConfig, FunctionConfigType, FunctionMap, FunctionType},
  seen_value::{SeenKey, SeenValue},
  state_manager::StateManager,
  theme_ref::ThemeRef,
};
use stylex_structures::named_import_source::ImportSources;
use stylex_utils::string::utf16_length;
use stylex_utils::{hash::stable_hash_unspanned, swc::get_expr_node_kind};

use crate::check_declaration::check_ident_declaration;

pub fn evaluate_obj_key(
  prop_kv: &KeyValueProp,
  state: &mut StateManager,
  functions: &FunctionMap,
) -> EvaluateResult {
  let key_path = &prop_kv.key;

  let key = match key_path {
    PropName::Ident(ident) => create_string_expr(&ident.sym),
    PropName::Computed(computed) => {
      let computed_result = evaluate(&computed.expr, state, functions);

      if !computed_result.confident {
        return EvaluateResult::refused(computed_result.deopt, computed_result.reason);
      }

      let Some(value) = computed_result
        .value
        .as_ref()
        .and_then(evaluate_result_as_expr)
      else {
        return EvaluateResult::refused(
          Some(*computed.expr.clone()),
          Some(KEY_HAS_NO_NAME.to_string()),
        );
      };

      value
    },

    PropName::Str(strng) => Expr::Lit(Lit::Str(strng.clone())),
    PropName::Num(num) => create_number_expr(num.value),
    PropName::BigInt(big_int) => create_big_int_expr(big_int.clone()),
  };

  let key_expr = match coercions::to_js_string(&key) {
    Some(ref text) => create_string_expr(text),
    None => return EvaluateResult::refused(Some(key), Some(KEY_HAS_NO_NAME.to_string())),
  };

  EvaluateResult {
    confident: true,
    deopt: None,
    reason: None,
    value: Some(EvaluateResultValue::Expr(key_expr)),
    inline_styles: None,
    fns: None,
  }
}

/// Folds `path` with a function map the caller owns.
pub fn evaluate(
  path: &Expr,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Box<EvaluateResult> {
  evaluate_with_functions(path, traversal_state, Rc::new(fns.clone()))
}

/// Folds `path` with a function map that is already shared.
pub fn evaluate_with_functions(
  path: &Expr,
  traversal_state: &mut StateManager,
  fns: Rc<FunctionMap>,
) -> Box<EvaluateResult> {
  let input_bytes = traversal_state.input_source_file.as_ref().map_or_else(
    || path.span().hi.0.saturating_sub(path.span().lo.0) as usize,
    |source| source.src.len(),
  );
  stylex_js::native::with_evaluation_sized(input_bytes, || {
    stylex_js::native::set_result_depth_limit(traversal_state.evaluation_ceiling());
    evaluate_with_functions_inner(path, traversal_state, fns)
  })
}
fn evaluate_with_functions_inner(
  path: &Expr,
  traversal_state: &mut StateManager,
  fns: Rc<FunctionMap>,
) -> Box<EvaluateResult> {
  traversal_state.keep_evaluation_functions(Rc::clone(&fns));
  let mut state = Box::new(EvaluationState::with_functions(Rc::clone(&fns)));

  let mut value = evaluate_cached(path, &mut state, traversal_state, &fns);
  traversal_state.replay_queued_evaluation_effects();

  if !state.confident {
    value = None;
  }

  Box::new(EvaluateResult {
    confident: state.confident,
    value,
    deopt: state.deopt_path,
    reason: state.deopt_reason,
    inline_styles: None,
    fns: None,
  })
}

fn _evaluate(
  path: &Expr,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Option<EvaluateResultValue> {
  if !state.confident {
    return None;
  }

  // The original `path` is kept beside it: a diagnostic must point at what the author wrote.
  let normalized_path = normalize_expr(path);

  if is_mutation_expr(normalized_path) {
    return deopt(path, state, NON_CONSTANT);
  }

  let result: Option<EvaluateResultValue> = match normalized_path {
    Expr::Arrow(arrow) => nodes::arrow_function::evaluate(arrow, state),

    Expr::Ident(ident) if binding::spells_the_env_import(ident, state, traversal_state) => None,
    Expr::Ident(ident) => nodes::identifier::evaluate(ident, state),
    Expr::TsSatisfies(ts_satisfaies) => {
      nodes::typescript_expression::evaluate(&ts_satisfaies.expr, state, traversal_state, fns)
    },
    Expr::TsConstAssertion(ts_const) => {
      nodes::typescript_expression::evaluate(&ts_const.expr, state, traversal_state, fns)
    },
    Expr::TsAs(ts_as) => {
      nodes::typescript_expression::evaluate(&ts_as.expr, state, traversal_state, fns)
    },
    Expr::TsNonNull(ts_non_null) => {
      nodes::typescript_expression::evaluate(&ts_non_null.expr, state, traversal_state, fns)
    },
    Expr::TsTypeAssertion(ts_type) => {
      nodes::typescript_expression::evaluate(&ts_type.expr, state, traversal_state, fns)
    },
    Expr::TsInstantiation(ts_instantiation) => {
      nodes::typescript_expression::evaluate(&ts_instantiation.expr, state, traversal_state, fns)
    },
    Expr::Seq(sec) => nodes::sequence_expression::evaluate(sec, state, traversal_state, fns),

    Expr::Lit(Lit::BigInt(_)) => {
      return deopt(
        normalized_path,
        state,
        &unsupported_expression("BigIntLiteral"),
      );
    },

    Expr::Lit(Lit::Regex(_)) => {
      return deopt(
        normalized_path,
        state,
        &unsupported_expression("RegExpLiteral"),
      );
    },
    Expr::Lit(lit_path) => nodes::literal::evaluate(lit_path),
    Expr::Tpl(tpl) => nodes::template_literal::evaluate_quasis(
      normalized_path,
      &tpl.exprs,
      &tpl.quasis,
      state,
      traversal_state,
      fns,
    ),
    Expr::TaggedTpl(_tagged_tpl) => {
      deopt_unsupported!(
        deopt,
        normalized_path,
        state,
        &unsupported_expression("TaggedTemplateExpression")
      )
      // TODO: Uncomment this for implementation of TaggedTpl
      // nodes::template_literal::evaluate_quasis(
      //   &Expr::TaggedTpl(_tagged_tpl.clone()),
      //   &_tagged_tpl.tpl.quasis,
      //   state,
      // )
    },
    Expr::Cond(cond) => nodes::conditional_expression::evaluate(cond, state, traversal_state, fns),
    Expr::Member(member) => nodes::member_expression::evaluate(member, state, traversal_state, fns),
    Expr::Unary(unary) => nodes::unary_expression::evaluate(unary, state, traversal_state, fns),
    Expr::Array(arr_path) => nodes::array_expression::evaluate(arr_path, state, traversal_state),
    Expr::Object(obj_path) => {
      nodes::object_expression::evaluate(obj_path, state, traversal_state, fns)
    },
    Expr::Bin(bin) => nodes::binary_expression::evaluate(bin, state, traversal_state, fns),
    Expr::Call(call) => nodes::call_expression::evaluate(call, state, traversal_state, fns),
    Expr::Await(await_expr) => {
      nodes::await_expression::evaluate(await_expr, state, traversal_state, fns)
    },
    Expr::OptChain(opt_chain) => {
      nodes::optional_chain::evaluate(opt_chain, state, traversal_state, fns)
    },
    _ => {
      let kind = get_expr_node_kind(normalized_path);

      warn!(
        "Unsupported type of expression: {kind}. For additional details, please recompile using debug mode."
      );

      debug!("Unsupported type of expression: {:?}", normalized_path);

      return deopt(normalized_path, state, &unsupported_expression(kind));
    },
  };

  if result.is_none()
    && let Some(ident) = normalized_path.as_ident()
  {
    return binding::resolve_reference(ident, path, normalized_path, state, traversal_state, fns);
  }

  if result.is_none() {
    return deopt(
      normalized_path,
      state,
      &unsupported_expression(get_expr_node_kind(normalized_path)),
    );
  }

  result
}

#[cfg(test)]
#[path = "tests/source_evaluation.rs"]
pub(crate) mod source_evaluation;

#[cfg(test)]
#[path = "tests/typescript_expression_tests.rs"]
mod typescript_expression_tests;

#[cfg(test)]
#[path = "tests/own_arrow_tests.rs"]
mod own_arrow_tests;

#[cfg(test)]
#[path = "tests/injected_function_map_tests.rs"]
mod injected_function_map_tests;

#[cfg(test)]
#[path = "tests/folded_function_callee_tests.rs"]
mod folded_function_callee_tests;

#[cfg(test)]
#[path = "tests/folded_member_read_tests.rs"]
mod folded_member_read_tests;

#[cfg(test)]
#[path = "tests/carried_value_tests.rs"]
mod carried_value_tests;

#[cfg(test)]
#[path = "tests/callback_parameter_tests.rs"]
mod callback_parameter_tests;

#[cfg(test)]
#[path = "tests/group_in_the_engine_tests.rs"]
mod group_in_the_engine_tests;

#[cfg(test)]
#[path = "tests/amplified_call_tests.rs"]
mod amplified_call_tests;

#[cfg(test)]
#[path = "tests/declined_call_dispatch_tests.rs"]
mod declined_call_dispatch_tests;

#[cfg(test)]
#[path = "tests/concatenation_chain_tests.rs"]
mod concatenation_chain_tests;

#[cfg(test)]
#[path = "tests/addend_reduction_tests.rs"]
mod addend_reduction_tests;

#[cfg(test)]
#[path = "tests/engine_stylex_function_tests.rs"]
mod engine_stylex_function_tests;

#[cfg(test)]
#[path = "tests/object_statics_over_a_declined_receiver_tests.rs"]
mod object_statics_over_a_declined_receiver_tests;

#[cfg(test)]
#[path = "tests/array_hole_tests.rs"]
mod array_hole_tests;

#[cfg(test)]
#[path = "tests/member_length_tests.rs"]
mod member_length_tests;

#[cfg(test)]
#[path = "tests/array_index_tests.rs"]
mod array_index_tests;

#[cfg(test)]
#[path = "tests/unsupported_shape_tests.rs"]
mod unsupported_shape_tests;

#[cfg(test)]
#[path = "tests/callee_shape_dispatch_tests.rs"]
mod callee_shape_dispatch_tests;

#[cfg(test)]
#[path = "tests/native_builtin_conformance.rs"]
mod native_builtin_conformance;

#[cfg(test)]
#[path = "tests/function_fold_object_tests.rs"]
mod function_fold_object_tests;

#[cfg(test)]
#[path = "tests/applied_global_tests.rs"]
mod applied_global_tests;

#[cfg(test)]
#[path = "tests/fall_through_tests.rs"]
mod fall_through_tests;

#[cfg(test)]
#[path = "tests/short_circuited_walk_tests.rs"]
mod short_circuited_walk_tests;

#[cfg(test)]
#[path = "tests/amplification_reading_tests.rs"]
mod amplification_reading_tests;

#[cfg(test)]
#[path = "tests/declined_call_receiver_tests.rs"]
mod declined_call_receiver_tests;

#[cfg(test)]
#[path = "tests/guarded_walk_tests.rs"]
mod guarded_walk_tests;

#[cfg(test)]
#[path = "tests/folded_answer_tests.rs"]
mod folded_answer_tests;

#[cfg(test)]
#[path = "tests/evaluated_array_form_tests.rs"]
mod evaluated_array_form_tests;

#[cfg(test)]
#[path = "tests/reported_message_tests.rs"]
mod reported_message_tests;

pub(crate) use callback_failure::with_isolated_causal_failures;
