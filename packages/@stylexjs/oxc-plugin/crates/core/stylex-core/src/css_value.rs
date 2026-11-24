//! Value-level CSS helpers, ported from #2008's `stylex-transform` with `Expr` replaced by
//! [`Value`]: `transform_value`, `flat_map_expanded_shorthands`, `convert_style_to_class_name`
//! and the object helpers `keyframes`, `positionTry` and `viewTransitionClass` share.

use std::borrow::Cow;

use indexmap::IndexMap;
use log::warn;
use stylex_constants::constants::messages::{ILLEGAL_PROP_VALUE, NON_CONTIGUOUS_VARS};
use stylex_css::css::common::{generate_css_rule, get_number_suffix, normalize_css_property_value};
use stylex_css::order::structures::{
  application_order::ApplicationOrder, legacy_expand_shorthands_order::LegacyExpandShorthandsOrder,
  property_specificity_order::PropertySpecificityOrder,
};
use stylex_css::utils::pre_rule::{sort_at_rules, sort_pseudos};
use stylex_css::values::content::transform_content_value;
use stylex_enums::{
  property_validation_mode::PropertyValidationMode, style_resolution::StyleResolution,
};
use stylex_macros::stylex_panic;
use stylex_structures::{
  order::Order, order_pair::OrderPair, pair::Pair, pre_rule_value::PreRuleValue,
  raw_value::TRawValue, stylex_state_options::StyleXStateOptions,
};
use stylex_types::structures::injectable_style::InjectableStyle;
use stylex_utils::{
  hash::create_hash,
  math::round_f64,
  number::to_js_string,
  string::{dashify, is_blank_css_text},
};
use stylex_value::Value;

use crate::StyleXCore;

/// The core's pre-rule value: #2008's `PreRuleValue` with a [`Value`] where it held an `Expr`.
pub(crate) type CorePreRuleValue = PreRuleValue<Value>;

/// Converts a CSS value in JS to the final CSS string value.
pub(crate) fn transform_value(key: &str, raw_value: &TRawValue, options: &StyleXStateOptions) -> String {
  let value = match raw_value.as_number() {
    Some(number) => {
      let mut value = to_js_string(round_f64(number, 4));
      value.push_str(get_number_suffix(key));
      value
    },
    None => raw_value.as_css_text().into_owned(),
  };

  if key == "content" || key == "hyphenateCharacter" || key == "hyphenate-character" {
    return transform_content_value(value.trim());
  }

  if is_blank_css_text(&value) {
    return value;
  }

  normalize_css_property_value(key, value.as_ref(), options)
}

impl StyleXCore {
  /// `transform_value` memoized per compile, keyed by JS identity (`1` and `'1'` differ).
  pub(crate) fn transform_value_cached(&mut self, key: &str, value: &TRawValue) -> String {
    let mut cache_key = String::with_capacity(key.len() + 26);
    cache_key.push_str(key);
    cache_key.push(':');
    value.write_identity_key(&mut cache_key);

    if let Some(result) = self.css_property_seen.get(&cache_key) {
      return result.clone();
    }
    let result = transform_value(key, value, &self.options);
    self.css_property_seen.insert(cache_key, result.clone());
    result
  }
}

fn validation_refusal(options: &StyleXStateOptions, msg: &str) -> Vec<OrderPair> {
  match options.property_validation_mode {
    PropertyValidationMode::Throw => stylex_panic!("{}", msg),
    PropertyValidationMode::Warn => {
      warn!("{}", msg);
      vec![]
    },
    PropertyValidationMode::Silent => vec![],
  }
}

/// The declarations one authored property stands for.
pub(crate) fn flat_map_expanded_shorthands(
  obj_entry: (Cow<'_, str>, CorePreRuleValue),
  options: &StyleXStateOptions,
) -> Vec<OrderPair> {
  let (key, raw_value) = obj_entry;

  let value = match raw_value {
    PreRuleValue::Raw(value) => Some(value),
    PreRuleValue::Vec(_) => {
      return validation_refusal(
        options,
        "Cannot use fallbacks for shorthands. Use the expansion instead.",
      );
    },
    PreRuleValue::Expr(value) => match value {
      Value::Number(n) => Some(TRawValue::Number(n)),
      Value::Null | Value::Bool(_) => None,
      Value::String(s) => Some(TRawValue::String(s)),
      _ => {
        return validation_refusal(
          options,
          "Cannot use expressions for shorthands. Use the expansion instead.",
        );
      },
    },
    PreRuleValue::Null => None,
  };

  let key = if key.starts_with("var(") && key.ends_with(')') {
    Cow::Owned(key[4..key.len() - 1].to_string())
  } else {
    key
  };

  let expansion_fn = match &options.style_resolution {
    StyleResolution::ApplicationOrder => ApplicationOrder::get_expansion_fn(key.as_ref()),
    StyleResolution::LegacyExpandShorthands => {
      LegacyExpandShorthandsOrder::get_expansion_fn(key.as_ref())
    },
    StyleResolution::PropertySpecificity => PropertySpecificityOrder::get_expansion_fn(key.as_ref()),
  };

  if let Some(expansion_fn) = expansion_fn {
    return match (expansion_fn)(value) {
      Ok(expanded) => expanded,
      Err(error_message) => validation_refusal(options, &error_message),
    };
  }

  vec![OrderPair(key.into_owned().into(), value)]
}

/// Compiles a resolved property/value pair into its class name and injected rule.
pub(crate) fn convert_style_to_class_name(
  core: &mut StyleXCore,
  obj_entry: (&str, &CorePreRuleValue),
  pseudos: &mut [String],
  at_rules: &mut [String],
  const_rules: &mut [String],
) -> Option<(String, String, InjectableStyle)> {
  let (key, raw_value) = obj_entry;

  let dashed_key = if key.starts_with("--") {
    Cow::Borrowed(key)
  } else {
    dashify(key)
  };

  let modifier_hash_string: Cow<'static, str> =
    if pseudos.is_empty() && at_rules.is_empty() && const_rules.is_empty() {
      Cow::Borrowed("null")
    } else {
      let sorted_pseudos = sort_pseudos(pseudos);
      let mut combined_at_rules = Vec::with_capacity(at_rules.len() + const_rules.len());
      combined_at_rules.extend_from_slice(at_rules);
      combined_at_rules.extend_from_slice(const_rules);
      let sorted_at_rules = sort_at_rules(&combined_at_rules);
      let at_rule_hash_string = sorted_at_rules.join("");
      let pseudo_hash_string = sorted_pseudos.join("");
      if at_rule_hash_string.is_empty() && pseudo_hash_string.is_empty() {
        Cow::Borrowed("null")
      } else {
        Cow::Owned(pseudo_hash_string + &at_rule_hash_string)
      }
    };

  let value: Vec<String> = match raw_value {
    PreRuleValue::Raw(raw_value) => vec![core.transform_value_cached(key, raw_value)],
    PreRuleValue::Vec(values) => {
      let values: Vec<String> = values
        .iter()
        .map(|each_value| core.transform_value_cached(key, each_value))
        .filter(|value| !is_blank_css_text(value))
        .collect();
      match values.iter().position(|value| is_css_var(value)) {
        Some(first_var) => variable_fallbacks(&values, first_var),
        None => values,
      }
    },
    PreRuleValue::Expr(_) | PreRuleValue::Null => stylex_panic!("{}", ILLEGAL_PROP_VALUE),
  };

  if value.iter().all(|text| is_blank_css_text(text)) {
    return None;
  }

  let mut string_to_hash = String::new();
  string_to_hash.push_str("<>");
  string_to_hash.push_str(dashed_key.as_ref());
  for (index, each_value) in value.iter().enumerate() {
    if index > 0 {
      string_to_hash.push_str(", ");
    }
    string_to_hash.push_str(each_value);
  }
  string_to_hash.push_str(&modifier_hash_string);

  let class_name_hashed = format!(
    "{}{}",
    core.options.class_name_prefix,
    create_hash(string_to_hash.as_str())
  );

  let css_rules = generate_css_rule(
    class_name_hashed.as_str(),
    dashed_key.as_ref(),
    &value,
    pseudos,
    at_rules,
    const_rules,
    &core.options,
  );

  Some((key.to_owned(), class_name_hashed, css_rules))
}

fn is_css_var(value: &str) -> bool {
  value.starts_with("var(") && value.ends_with(')')
}

fn variable_fallbacks(values: &[String], first_var: usize) -> Vec<String> {
  let last_var = first_var
    + values[first_var..]
      .iter()
      .rposition(|value| is_css_var(value))
      .unwrap_or_default();

  let values_before_first_var = &values[..first_var];
  let var_values: Vec<String> = values[first_var..=last_var].iter().rev().cloned().collect();
  let values_after_last_var = &values[last_var + 1..];

  if !var_values.iter().all(|value| is_css_var(value)) {
    stylex_panic!("{}", NON_CONTIGUOUS_VARS);
  }

  let var_values: Vec<String> = var_values
    .iter()
    .map(|val| val[4..val.len() - 1].to_string())
    .collect();

  let mut result = Vec::new();
  if !values_before_first_var.is_empty() {
    for val in values_before_first_var {
      let mut to_push = var_values.clone();
      to_push.push(val.clone());
      result.push(compose_vars(&to_push[0], &to_push[1..]));
    }
  } else {
    result.push(compose_vars(&var_values[0], &var_values[1..]));
  }
  result.extend(values_after_last_var.iter().cloned());
  result
}

fn compose_vars(first: &str, rest: &[String]) -> String {
  if let Some((next, tail)) = rest.split_first() {
    format!("var({},{})", first, compose_vars(next, tail))
  } else if first.starts_with("--") {
    format!("var({first})")
  } else {
    first.to_string()
  }
}

/// `preprocess_object_properties`: an object's entries, shorthands expanded, values kept raw.
pub(crate) fn preprocess_object_properties(
  style: &stylex_value::Object,
  options: &StyleXStateOptions,
) -> IndexMap<String, TRawValue> {
  let mut map = IndexMap::new();
  for (key, value) in style {
    for OrderPair(property, value) in
      flat_map_expanded_shorthands((Cow::Borrowed(key.as_str()), PreRuleValue::Expr(value.clone())), options)
    {
      if let Some(value) = value {
        map.insert(property.into_owned(), value);
      }
    }
  }
  map
}

impl StyleXCore {
  /// `obj_map_keys_and_transform_values`: map the key, then transform under the mapped key.
  pub(crate) fn map_keys_then_transform(
    &mut self,
    entries: &IndexMap<String, TRawValue>,
    mapper: impl Fn(&str) -> String,
  ) -> Vec<Pair> {
    let mut map: IndexMap<String, String> = IndexMap::with_capacity(entries.len());
    for (key, value) in entries {
      let mapped_key = mapper(key);
      let value = self.transform_value_cached(mapped_key.as_str(), value);
      map.insert(mapped_key, value);
    }
    map.into_iter().map(|(key, value)| Pair { key, value }).collect()
  }

  /// `obj_transform_values_then_map_keys`: transform under the authored key, then map it.
  pub(crate) fn transform_then_map_keys(
    &mut self,
    entries: &IndexMap<String, TRawValue>,
    mapper: impl Fn(&str) -> String,
  ) -> Vec<Pair> {
    let mut map: IndexMap<String, String> = IndexMap::with_capacity(entries.len());
    for (key, value) in entries {
      let transformed = self.transform_value_cached(key, value);
      map.insert(mapper(key), transformed);
    }
    map.into_iter().map(|(key, value)| Pair { key, value }).collect()
  }
}
