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
use stylex_structures::{
  pair::{Pair, PairCow},
  stylex_state_options::with_default_options,
};
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

  // Both directions read the shared default options, which is what keeps a
  // position-try name the same however the module is compiled.
  let (ltr_string, rtl_string) = with_default_options(|state_options| {
    let ltr_string = construct_position_try_obj(&declarations, |pair| {
      doubled_css_text(&pair.key, &generate_ltr(pair, state_options))
    });

    // When no RTL transform applies, fall back to the bare value, which
    // serializes to `key:value;` rather than the doubled `key:key;key:value;`
    // form produced for a resolved declaration.
    let rtl_string = construct_position_try_obj(&declarations, |pair| {
      match generate_rtl(pair, state_options) {
        Some(rtl_value) => doubled_css_text(&pair.key, &rtl_value),
        None => pair.as_css_text(),
      }
    });

    (ltr_string, rtl_string)
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

/// The CSS text a direction-resolved value contributes to `property`, or `None`
/// when it contributes none.
///
/// `resolved` does not hold a key and a value despite its type: both halves are
/// *values* for the same `property`, and the reference implementation emits them
/// doubled as `property:first;property:second;`, where the first half repeats
/// the property name as its own value. That repeat is a companion to the real
/// value rather than CSS text in its own right, so a real value spelling nothing
/// takes the repeat with it -- otherwise a stray `top:top;` survives a dropped
/// `top` and moves the name the body is hashed into.
fn doubled_css_text(property: &str, resolved: &PairCow) -> Option<String> {
  let value = Pair::new(property, resolved.value.as_ref()).as_css_text()?;

  // The repeat is written from the key the resolver named, and is not asked
  // whether it spells text. `generate_ltr` and `generate_rtl` always name the
  // property they resolved, so the only half that can spell nothing is the
  // value, which is read above. What goes unchecked here is that invariant.
  let mut css_text = String::with_capacity(property.len() + resolved.key.len() + 2 + value.len());
  css_text.push_str(property);
  css_text.push(':');
  css_text.push_str(&resolved.key);
  css_text.push(';');
  css_text.push_str(&value);

  Some(css_text)
}

/// The body of one `@position-try` rule, its declarations written in the order
/// their property names sort in.
///
/// `resolve` answers the CSS text a declaration contributes in the direction
/// the caller asks for, and `None` where it contributes none.
fn construct_position_try_obj(
  declarations: &[Pair],
  resolve: impl Fn(&Pair) -> Option<String>,
) -> String {
  let mut sorted = declarations.iter().collect::<Vec<&Pair>>();
  sorted.sort_unstable_by(|left, right| left.key.cmp(&right.key));

  let mut output = String::with_capacity(sorted.len().saturating_mul(32));

  for declaration in sorted {
    if let Some(css_text) = resolve(declaration) {
      output.push_str(&css_text);
    }
  }

  output
}
