use std::borrow::Cow;

use stylex_css::css::common::generate_css_rule;
use stylex_macros::stylex_panic;
use stylex_structures::pre_rule_value::PreRuleValue;
use stylex_types::structures::style_key::{ClassName, RuleKey};

use crate::shared::utils::css::common::transform_value_cached;
use stylex_constants::constants::messages::{ILLEGAL_PROP_VALUE, NON_CONTIGUOUS_VARS};
use stylex_css::utils::pre_rule::{sort_at_rules, sort_pseudos};
use stylex_state::state_manager::StateManager;
use stylex_types::structures::injectable_style::InjectableStyle;
use stylex_utils::{
  hash::create_hash,
  string::{dashify, is_blank_css_text},
};

/// Compiles a resolved property/value pair into its class name and injected rule.
/// `None` when the *transformed* value is blank (e.g. `color:`); a blank `content` becomes `""`.
pub(crate) fn convert_style_to_class_name(
  obj_entry: (&str, &PreRuleValue),
  pseudos: &mut [String],
  at_rules: &mut [String],
  const_rules: &mut [String],
  state: &mut StateManager,
) -> Option<(RuleKey, ClassName, InjectableStyle)> {
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
        // NOTE: 'null' is used to keep existing hashes stable. Remove in a future version.
        Cow::Borrowed("null")
      } else {
        // TODO: set correct order when will be answer from the Meta team
        // Link to discussion: https://github.com/facebook/stylex/discussions/744
        Cow::Owned(pseudo_hash_string + &at_rule_hash_string)
      }
    };

  let value: Vec<String> = match raw_value {
    PreRuleValue::Raw(raw_value) => vec![transform_value_cached(key, raw_value, state)],
    PreRuleValue::Vec(values) => {
      // Drop blank entries before hashing and before `variable_fallbacks`, which needs
      // contiguous `var()` entries.
      let values: Vec<String> = values
        .iter()
        .map(|each_value| transform_value_cached(key, each_value, state))
        .filter(|value| !is_blank_css_text(value))
        .collect();

      match values.iter().position(|value| is_css_var(value)) {
        Some(first_var) => variable_fallbacks(&values, first_var),
        None => values,
      }
    },
    PreRuleValue::Expr(_) | PreRuleValue::Null => stylex_panic!("{}", ILLEGAL_PROP_VALUE),
  };

  // Fallback arrays arrive without blanks, so an emptied array is also all-blank.
  if value.iter().all(|text| is_blank_css_text(text)) {
    return None;
  }

  // `<>`, key, values joined by `, `, modifier, in one presized buffer.
  let values_len = value.iter().map(String::len).sum::<usize>() + 2 * value.len().saturating_sub(1);
  let mut string_to_hash =
    String::with_capacity(2 + dashed_key.len() + values_len + modifier_hash_string.len());
  string_to_hash.push_str("<>");
  string_to_hash.push_str(dashed_key.as_ref());
  for (index, each_value) in value.iter().enumerate() {
    if index > 0 {
      string_to_hash.push_str(", ");
    }
    string_to_hash.push_str(each_value);
  }
  string_to_hash.push_str(&modifier_hash_string);

  let prefix = &state.options.class_name_prefix;
  let hash = create_hash(string_to_hash.as_str());

  let mut class_name_hashed = String::with_capacity(prefix.len() + hash.len());
  class_name_hashed.push_str(prefix);
  class_name_hashed.push_str(&hash);

  let css_rules = generate_css_rule(
    class_name_hashed.as_str(),
    dashed_key.as_ref(),
    &value,
    pseudos,
    at_rules,
    const_rules,
    &state.options,
  );

  Some((
    RuleKey::from(key),
    ClassName::from(class_name_hashed),
    css_rules,
  ))
}

/// Whether a value is one whole `var()` reference (looser than `IS_CSS_VAR`, matching the
/// reference implementation).
fn is_css_var(value: &str) -> bool {
  value.starts_with("var(") && value.ends_with(')')
}

/// The fallback chain a set of values spells, given the position of its first `var()` entry.
fn variable_fallbacks(values: &[String], first_var: usize) -> Vec<String> {
  let last_var = first_var
    + values[first_var..]
      .iter()
      .rposition(|value| is_css_var(value))
      .unwrap_or_default();

  let values_before_first_var = &values[..first_var];

  // Non-empty: `first_var <= last_var`.
  let mut var_values: Vec<String> = values[first_var..=last_var]
    .iter()
    .rev()
    .cloned()
    .collect::<Vec<String>>();

  let values_after_last_var = &values[last_var + 1..];

  if !var_values.iter().all(|value| is_css_var(value)) {
    stylex_panic!("{}", NON_CONTIGUOUS_VARS);
  }

  var_values = var_values
    .iter()
    .map(|val| val[4..val.len() - 1].to_string())
    .collect::<Vec<String>>();

  let result_capacity = if values_before_first_var.is_empty() {
    1
  } else {
    values_before_first_var.len()
  } + values_after_last_var.len();
  let mut result = Vec::with_capacity(result_capacity);

  if !values_before_first_var.is_empty() {
    // Lay down the shared var prefix once and swap only the trailing value.
    let mut to_push = Vec::with_capacity(var_values.len() + 1);
    to_push.extend_from_slice(&var_values);
    to_push.push(String::new());

    let trailing = to_push.len() - 1;

    for val in values_before_first_var {
      to_push[trailing].clear();
      to_push[trailing].push_str(val);

      result.push(compose_vars(&to_push[0], &to_push[1..]));
    }
  } else {
    result.push(compose_vars(&var_values[0], &var_values[1..]));
  }

  for val in values_after_last_var {
    result.push(val.to_string());
  }

  result
}

/// The `var(a, var(b, c))` chain a set of values spells.
fn compose_vars(first: &str, rest: &[String]) -> String {
  if let Some((next, tail)) = rest.split_first() {
    let fallback = compose_vars(next, tail);
    let mut result = String::with_capacity(first.len() + fallback.len() + 6);
    result.push_str("var(");
    result.push_str(first);
    result.push(',');
    result.push_str(&fallback);
    result.push(')');
    result
  } else if first.starts_with("--") {
    let mut result = String::with_capacity(first.len() + 5);
    result.push_str("var(");
    result.push_str(first);
    result.push(')');
    result
  } else {
    first.to_string()
  }
}
