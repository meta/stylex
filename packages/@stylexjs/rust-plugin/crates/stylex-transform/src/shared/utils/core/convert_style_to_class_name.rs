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

/// Compiles a resolved property/value pair into the class name that carries it
/// and the rule that class name injects.
///
/// `None` when the transformed value carries no CSS text: the declaration would
/// be `color:`, which is invalid CSS a browser discards, so the property is left
/// undeclared instead. The test is on the *transformed* value rather than the
/// authored one, because transformation is what decides whether a blank value
/// spells anything -- a blank `content` is quoted into `""`, which does.
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

  // `sort_pseudos` takes a slice and copies what it needs; copying into a
  // binding it never mutates first was a second copy of the list, per property
  // per namespace per call.
  let sorted_pseudos = sort_pseudos(pseudos);

  let mut combined_at_rules = Vec::with_capacity(at_rules.len() + const_rules.len());

  combined_at_rules.extend_from_slice(at_rules);
  combined_at_rules.extend_from_slice(const_rules);

  let sorted_at_rules = sort_at_rules(&combined_at_rules);

  let at_rule_hash_string = sorted_at_rules.join("");
  let pseudo_hash_string = sorted_pseudos.join("");

  let modifier_hash_string = if at_rule_hash_string.is_empty() && pseudo_hash_string.is_empty() {
    // NOTE: 'null' is used to keep existing hashes stable.
    // This should be removed in a future version.
    "null".to_string()
  } else {
    // TODO: set correct order when will be answer from the Meta team
    // Link to discussion: https://github.com/facebook/stylex/discussions/744
    format!("{}{}", pseudo_hash_string, at_rule_hash_string)
  };

  let value: Vec<String> = match raw_value {
    PreRuleValue::Raw(raw_value) => vec![transform_value_cached(key, raw_value, state)],
    PreRuleValue::Vec(values) => {
      // A blank entry drops before the fallback chain is built, so the class
      // name is hashed from the entries that survive: a blank beside `red`
      // yields the class name a lone `red` yields. It also has to go before
      // `variable_fallbacks`, which requires the `var()` entries it composes to
      // be contiguous.
      let values: Vec<String> = values
        .iter()
        .map(|each_value| transform_value_cached(key, each_value, state))
        .filter(|value| !is_blank_css_text(value))
        .collect();

      // The position is what the composer needs, and asking for it answers the
      // same question as asking whether there is one. Handing it over is what
      // lets the composer read a set it is told holds a `var()` entry, rather
      // than defaulting each end to a position no call reaches.
      match values.iter().position(|value| is_css_var(value)) {
        Some(first_var) => variable_fallbacks(&values, first_var),
        None => values,
      }
    },
    PreRuleValue::Expr(_) | PreRuleValue::Null => stylex_panic!("{}", ILLEGAL_PROP_VALUE),
  };

  // A lone value is judged here; a fallback array arrives with its blank
  // entries already gone, so `all` over no values -- also `true` -- is what
  // answers for an array that emptied.
  if value.iter().all(|text| is_blank_css_text(text)) {
    return None;
  }

  let string_to_hash = format!(
    "<>{}{}{}",
    dashed_key.as_ref(),
    value.join(", "),
    modifier_hash_string
  );

  let prefix = &state.options.class_name_prefix;

  let class_name_hashed = format!("{}{}", prefix, create_hash(string_to_hash.as_str()));

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

/// Whether a value is one whole `var()` reference.
///
/// Not the `IS_CSS_VAR` expression, and the difference is deliberate. That one
/// asks for a custom property name -- `var(--name)`, with the name spelled from
/// a fixed set of characters -- because the caller it serves asks a different
/// question. The fallback chain asks only that the value opens with `var(` and
/// closes the bracket, which is the test the reference makes here, so a
/// `var(x)` is a link in the chain to both compilers.
fn is_css_var(value: &str) -> bool {
  value.starts_with("var(") && value.ends_with(')')
}

/// The fallback chain a set of values spells, given the position of its first
/// `var()` entry.
///
/// The position comes from the caller, which found it deciding whether there
/// was a chain to build at all. That makes the entry part of what this reads
/// rather than something it has to look for and then account for not finding.
fn variable_fallbacks(values: &[String], first_var: usize) -> Vec<String> {
  // `values[first_var]` is a `var()` entry, so a search back over the tail that
  // starts with it always answers -- and 0, the entry itself, is the answer for
  // a tail holding only that one.
  let last_var = first_var
    + values[first_var..]
      .iter()
      .rposition(|value| is_css_var(value))
      .unwrap_or_default();

  let values_before_first_var = &values[..first_var];

  // Non-empty: `first_var <= last_var`, so the range holds at least the entry
  // the caller found.
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
    // The var prefix is the same for every iteration, so it is laid down once
    // and only the trailing value is swapped — rather than cloning the whole
    // prefix per value, which is what this cost before.
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
    // At least one entry, for the reason the slice above states.
    result.push(compose_vars(&var_values[0], &var_values[1..]));
  }

  for val in values_after_last_var {
    result.push(val.to_string());
  }

  result
}

/// The `var(a, var(b, c))` chain a set of values spells.
///
/// The first value is asked for on its own, so a caller cannot spell a chain of
/// nothing: [`variable_fallbacks`] slices a set that holds at least the `var()`
/// entry its caller found, and the recursion goes on only while there is a rest
/// to go on with.
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
