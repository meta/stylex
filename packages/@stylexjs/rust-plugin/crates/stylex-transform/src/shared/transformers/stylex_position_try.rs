use crate::shared::transformers::named_rule::fold_to_rule_name;
use crate::shared::utils::object::{
  obj_map_keys_and_transform_values, preprocess_object_properties,
};
use stylex_constants::constants::messages::VALUES_MUST_BE_OBJECT;
use stylex_css::css::{generate_ltr::generate_ltr, generate_rtl::generate_rtl};
use stylex_macros::stylex_panic;
use stylex_state::{
  evaluate_result_value::EvaluateResultValue,
  functions::{FunctionConfig, FunctionType},
  state_manager::StateManager,
};
use stylex_structures::pair::{Pair, PairCow};
use stylex_types::{
  enums::data_structures::injectable_style::InjectableStyleKind,
  structures::injectable_style::InjectableStyle,
};
use stylex_utils::{hash::create_hash, string::dashify};

pub(crate) fn stylex_position_try(
  styles: &EvaluateResultValue,
  state: &mut StateManager,
) -> (String, InjectableStyleKind) {
  // NOTE: an unset `classNamePrefix` arrives here already defaulted to `x`,
  // so an empty one was asked for explicitly and is honoured as empty.
  let class_name_prefix = state.options.class_name_prefix.clone();

  let Some(styles) = styles.as_expr().filter(|expr| expr.is_object()) else {
    stylex_panic!("{}", VALUES_MUST_BE_OBJECT)
  };

  let entries = preprocess_object_properties(styles, state);
  let declarations =
    obj_map_keys_and_transform_values(&entries, state, |key| dashify(key).into_owned());

  // Directional generators return a key/value pair. Serialize it once and
  // honor this call's normalized options; a flipped key is not a fallback value.
  let options = &state.options;
  let ltr_string = construct_position_try_obj(&declarations, |pair| generate_ltr(pair, options));
  let rtl_string = construct_position_try_obj(&declarations, |pair| {
    generate_rtl(pair, options).unwrap_or_else(|| PairCow::borrowed(pair))
  });

  let position_try_name = format!("--{}{}", class_name_prefix, create_hash(&ltr_string));

  // The space before the block is part of the rule text the reference
  // implementation emits. It sits outside the hash, which is taken from the body
  // alone, so the name is unaffected either way -- but the rule text is compared
  // byte for byte wherever both compilers' output meets.
  let ltr = format!("@position-try {} {{{}}}", position_try_name, ltr_string);
  let rtl = if ltr_string == rtl_string {
    None
  } else {
    Some(format!(
      "@position-try {} {{{}}}",
      position_try_name, rtl_string
    ))
  };

  (
    position_try_name,
    InjectableStyleKind::Regular(InjectableStyle {
      ltr,
      rtl,
      priority: Some(0.0),
    }),
  )
}

pub(crate) fn get_position_try_fn() -> FunctionConfig {
  FunctionConfig {
    fn_ptr: FunctionType::StylexExprFn(|expr, state| {
      fold_to_rule_name(expr, state, stylex_position_try)
    }),
    takes_path: false,
  }
}

/// Map each directional key, collapse duplicates like Object.fromEntries,
/// then sort the resulting names by JavaScript's UTF-16 ordering.
fn construct_position_try_obj(
  declarations: &[Pair],
  resolve: impl for<'a> Fn(&'a Pair) -> PairCow<'a>,
) -> String {
  let mut mapped = indexmap::IndexMap::with_capacity(declarations.len());
  for declaration in declarations {
    let pair = resolve(declaration);
    mapped.insert(pair.key.into_owned(), pair.value.into_owned());
  }
  let mut sorted = mapped.into_iter().collect::<Vec<_>>();
  sorted.sort_unstable_by(|(left, _), (right, _)| left.encode_utf16().cmp(right.encode_utf16()));
  let mut output = String::with_capacity(sorted.len().saturating_mul(32));
  for (key, value) in sorted {
    if let Some(css_text) = Pair::new(key, value).as_css_text() {
      output.push_str(&css_text);
    }
  }
  output
}
