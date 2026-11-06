use std::borrow::Cow;

use indexmap::IndexMap;
use stylex_macros::stylex_panic;
use stylex_structures::pre_rule_value::PreRuleValue;
use swc_core::ecma::ast::{Expr, Lit};

use crate::shared::transformers::named_rule::fold_to_rule_name;
use crate::shared::utils::{
  core::flat_map_expanded_shorthands::flat_map_expanded_shorthands,
  object::{obj_entries, obj_from_entries, obj_map_keys_and_transform_values},
};
use stylex_ast::ast::convertors::{
  convert_key_value_to_str, get_key_values_from_object, normalize_expr,
};
use stylex_constants::constants::messages::VALUES_MUST_BE_OBJECT;
use stylex_css::css::{generate_ltr::generate_ltr, generate_rtl::generate_rtl};
use stylex_state::resolution::convertors::convert_expr_to_str;
use stylex_state::{
  evaluate_result_value::EvaluateResultValue,
  functions::{FunctionConfig, FunctionMap, FunctionType},
  state_manager::StateManager,
};
use stylex_structures::{
  order_pair::OrderPair,
  pair::{Pair, PairCow},
  raw_value::TRawValue,
  stylex_state_options::with_default_options,
};
use stylex_types::{
  enums::data_structures::injectable_style::InjectableStyleKind,
  structures::injectable_style::InjectableStyle,
};
use stylex_utils::{hash::create_hash, string::dashify};

/// One animation step: the offset it is written under, and the declarations
/// that step makes.
type KeyframeSteps = IndexMap<String, Vec<Pair>>;

pub(crate) fn stylex_keyframes(
  frames: &EvaluateResultValue,
  state: &mut StateManager,
) -> (String, InjectableStyleKind) {
  let Some(frames) = frames.as_expr().and_then(|expr| expr.as_object()) else {
    stylex_panic!("{}", VALUES_MUST_BE_OBJECT)
  };

  let key_values = get_key_values_from_object(frames);

  // One entry per animation step, holding the declarations that step makes. A
  // step written twice is one step, and the declarations written last stand.
  // Each direction below reads these same pairs, so the shorthand expansion is
  // done once.
  let mut expanded_steps = KeyframeSteps::with_capacity(key_values.len());

  for key_value in key_values.iter() {
    let step = convert_key_value_to_str(key_value);
    let entries = expand_frame_shorthands(&key_value.value, state);

    expanded_steps.insert(
      step,
      obj_map_keys_and_transform_values(&entries, state, |key| dashify(key).into_owned()),
    );
  }

  // Read rather than copied. The steps above are the last work that writes
  // into the state, so the options can be borrowed for the rest of the call.
  // NOTE: an unset `classNamePrefix` arrives here already defaulted to `x`,
  // so an empty one was asked for explicitly and is honoured as empty.
  let options = &state.options;
  let class_name_prefix = &options.class_name_prefix;

  let ltr_string = construct_keyframes_obj(&expanded_steps, |pair| generate_ltr(pair, options));

  // The name is hashed from what the default options resolve, so that it holds
  // whatever options the module is compiled with. The defaults are shared, so
  // nothing is built here.
  let stable_string = with_default_options(|stable_options| {
    construct_keyframes_obj(&expanded_steps, |pair| generate_ltr(pair, stable_options))
  });

  // A declaration with no right-to-left form keeps the one it was written with.
  let rtl_string = construct_keyframes_obj(&expanded_steps, |pair| {
    generate_rtl(pair, options).unwrap_or_else(|| PairCow::borrowed(pair))
  });

  // NOTE: Use a direction-agnostic hash to keep LTR/RTL classnames stable across
  // builds. NOTE: '<>' and '-B' is used to keep existing hashes stable.
  // TODO: They should be removed in a future version.
  let animation_name = format!(
    "{}{}-B",
    class_name_prefix,
    create_hash(&format!("<>{}", stable_string))
  );

  let ltr = format!("@keyframes {}{{{}}}", animation_name, ltr_string);
  let rtl = if ltr_string == rtl_string {
    None
  } else {
    Some(format!("@keyframes {}{{{}}}", animation_name, rtl_string))
  };

  (
    animation_name,
    InjectableStyleKind::Regular(InjectableStyle {
      ltr,
      rtl,
      priority: Some(0.0),
    }),
  )
}

/// The steps of one keyframes rule, written as `step{declarations}`.
///
/// `resolve` answers the declaration a pair makes in the direction the caller
/// asks for, and a pair that spells nothing is dropped.
fn construct_keyframes_obj(
  steps: &KeyframeSteps,
  resolve: impl for<'a> Fn(&'a Pair) -> PairCow<'a>,
) -> String {
  let mut result = String::with_capacity(steps.len() * 32);

  for (step, pairs) in steps {
    result.push_str(step);
    result.push('{');

    for css_text in pairs.iter().filter_map(|pair| resolve(pair).as_css_text()) {
      result.push_str(&css_text);
    }

    result.push('}');
  }

  result
}

fn expand_frame_shorthands(frame: &Expr, state: &mut StateManager) -> IndexMap<String, TRawValue> {
  let res: Vec<_> = obj_entries(frame)
    .iter()
    .flat_map(|pair| {
      let key = convert_key_value_to_str(pair);
      // A numeric frame value keeps its JS type all the way to
      // `transform_value`, which is what appends the unit suffix; only a
      // non-numeric value is coerced to a string here.
      let value = match normalize_expr(pair.value.as_ref()) {
        Expr::Lit(Lit::Num(num)) => Some(PreRuleValue::number(num.value)),
        _ => convert_expr_to_str(pair.value.as_ref(), state, &FunctionMap::default())
          .map(PreRuleValue::string),
      };

      // A step value that is not a string or a number declares nothing. An
      // animation step has no condition to apply and no fallback to choose
      // from, so a nested value object and a fallback array mean nothing here,
      // and neither does `null` -- the step keeps whatever else it declares.
      let Some(value) = value else {
        return vec![];
      };

      flat_map_expanded_shorthands((Cow::Owned(key), value), &state.options)
        .into_iter()
        .filter_map(|pair| {
          pair.1.as_ref()?;

          Some(pair)
        })
        .collect::<Vec<OrderPair>>()
    })
    .filter(|item| item.1.is_some())
    .collect::<Vec<OrderPair>>();

  obj_from_entries(&res)
}

pub(crate) fn get_keyframes_fn() -> FunctionConfig {
  FunctionConfig {
    fn_ptr: FunctionType::StylexExprFn(|expr, state| {
      fold_to_rule_name(expr, state, stylex_keyframes)
    }),
    takes_path: false,
  }
}
