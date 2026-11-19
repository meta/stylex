use std::borrow::Cow;

use indexmap::IndexMap;
use stylex_macros::stylex_panic;
use stylex_structures::pre_rule_value::PreRuleValue;
use swc_core::ecma::ast::{Expr, Lit};

use crate::shared::transformers::named_rule::fold_to_rule_name;
use crate::shared::utils::{
  core::flat_map_expanded_shorthands::flat_map_expanded_shorthands,
  object::{obj_entries, obj_from_entries, obj_transform_values_then_map_keys},
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

/// Animation steps: offset to the declarations at that step.
type KeyframeSteps = IndexMap<String, Vec<Pair>>;

pub(crate) fn stylex_keyframes(
  frames: &EvaluateResultValue,
  state: &mut StateManager,
) -> (String, InjectableStyleKind) {
  let Some(frames) = frames.as_expr().and_then(|expr| expr.as_object()) else {
    stylex_panic!("{}", VALUES_MUST_BE_OBJECT)
  };

  let key_values = get_key_values_from_object(frames);

  // A repeated step merges; the last declaration wins. Shorthands are expanded once.
  let mut expanded_steps = KeyframeSteps::with_capacity(key_values.len());

  for key_value in key_values.iter() {
    let step = convert_key_value_to_str(key_value);
    let entries = expand_frame_shorthands(&key_value.value, state);

    expanded_steps.insert(
      step,
      obj_transform_values_then_map_keys(&entries, state, |key| dashify(key).into_owned()),
    );
  }

  // `classNamePrefix` is already defaulted to `x`; an empty one is honoured as empty.
  let options = &state.options;
  let class_name_prefix = &options.class_name_prefix;

  let ltr_string = construct_keyframes_obj(&expanded_steps, |pair| generate_ltr(pair, options));

  // Hash from the default options so the name is independent of compile options.
  let stable_string = with_default_options(|stable_options| {
    construct_keyframes_obj(&expanded_steps, |pair| generate_ltr(pair, stable_options))
  });

  // A declaration with no RTL form keeps its LTR form.
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

/// The steps of one keyframes rule as `step{declarations}`; pairs `resolve` drops are omitted.
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
      // Numbers stay numbers so `transform_value` can append the unit suffix.
      let value = match normalize_expr(pair.value.as_ref()) {
        Expr::Lit(Lit::Num(num)) => Some(PreRuleValue::number(num.value)),
        _ => convert_expr_to_str(pair.value.as_ref(), state, &FunctionMap::default())
          .map(PreRuleValue::string),
      };

      // Non-string, non-number values (objects, arrays, `null`) declare nothing in a step.
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
