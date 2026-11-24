//! `stylex.create` over plain values: flatten, pre-rules, class names and rules, and the
//! dynamic-style placeholder pass. Ported from #2008's `flatten_raw_style_object.rs`,
//! `pre_rule*.rs`, `stylex_create.rs` and `evaluate_stylex_create_arg.rs`.

use std::borrow::Cow;

use indexmap::{IndexMap, IndexSet};
use rustc_hash::FxHashSet;
use stylex_constants::constants::{
  length_units::LENGTH_UNITS,
  messages::{ILLEGAL_PROP_ARRAY_VALUE, ILLEGAL_PROP_VALUE, INVALID_MEDIA_QUERY_SYNTAX, non_static_value},
  time_units::get_time_units,
};
use stylex_css::css::common::get_number_suffix;
use stylex_css::utils::{
  condition::is_conditional_key,
  pre_rule::{sort_at_rules, sort_pseudos},
  pseudo::is_pseudo_selector,
};
use stylex_css_parser::at_queries::media_query_transform::last_media_query_wins_transform_value;
use stylex_macros::stylex_panic;
use stylex_regex::regex::CSS_VAR_REFERENCE;
use stylex_structures::{order_pair::OrderPair, pre_rule_value::PreRuleValue, raw_value::TRawValue};
use stylex_types::structures::injectable_style::InjectableStyle;
use stylex_utils::hash::create_hash;
use stylex_value::{Object, OpaqueId, Value};

use crate::{
  StyleXCore,
  css_value::{CorePreRuleValue, convert_style_to_class_name, flat_map_expanded_shorthands},
};

// ---------------------------------------------------------------------------------------------
// Pre-rules
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ComputedStyle {
  pub class_name: String,
  pub style: InjectableStyle,
  pub original_path: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PreRules {
  Set(Vec<PreRules>),
  Styles(StylesPreRule),
  Null,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StylesPreRule {
  property: String,
  value: CorePreRuleValue,
  pseudos: Vec<String>,
  at_rules: Vec<String>,
  const_rules: Vec<String>,
  key_path: Vec<String>,
}

impl StylesPreRule {
  fn select(key_path: &[String], keep: impl Fn(&str) -> bool) -> Vec<String> {
    key_path.iter().filter(|key| keep(key)).cloned().collect()
  }

  pub(crate) fn new(property: &str, value: CorePreRuleValue, key_path: Vec<String>) -> Self {
    StylesPreRule {
      property: property.to_string(),
      value,
      pseudos: sort_pseudos(&Self::select(&key_path, |key| {
        is_pseudo_selector(key) || key.starts_with('[')
      })),
      at_rules: sort_at_rules(&Self::select(&key_path, |key| key.starts_with('@'))),
      const_rules: Self::select(&key_path, |key| key.starts_with("var(--")),
      key_path,
    }
  }
}

impl PreRules {
  fn create_set(rules: Vec<PreRules>) -> PreRules {
    let mut flat: Vec<PreRules> = rules
      .into_iter()
      .flat_map(|rule| match rule {
        PreRules::Set(rules) => rules,
        other => vec![other],
      })
      .collect();
    match flat.len() {
      0 => PreRules::Null,
      1 => flat.swap_remove(0),
      _ => PreRules::Set(flat),
    }
  }

  /// `None` is #2008's `CompiledResult::Null`.
  pub(crate) fn compiled(&mut self, core: &mut StyleXCore) -> Option<Vec<ComputedStyle>> {
    match self {
      PreRules::Null => None,
      PreRules::Set(rules) => Some(
        rules
          .iter_mut()
          .flat_map(|rule| rule.compiled(core).unwrap_or_default())
          .collect(),
      ),
      PreRules::Styles(rule) => {
        let (_, class_name, style) = convert_style_to_class_name(
          core,
          (rule.property.as_str(), &rule.value),
          &mut rule.pseudos,
          &mut rule.at_rules,
          &mut rule.const_rules,
        )?;
        Some(vec![ComputedStyle {
          class_name,
          style,
          original_path: rule.key_path.clone(),
        }])
      },
    }
  }
}

// ---------------------------------------------------------------------------------------------
// Flatten
// ---------------------------------------------------------------------------------------------

fn normalize_key_path(key_path: Vec<String>, key: &str, property: &str) -> Vec<String> {
  if key_path.iter().any(|step| step == key) {
    key_path
      .into_iter()
      .map(|step| if step == key { property.to_string() } else { step })
      .collect()
  } else {
    let mut new_key_path = key_path;
    new_key_path.push(property.to_string());
    new_key_path
  }
}

fn var_reference_name(key: &str) -> String {
  let units = |text: &str| text.chars().map(char::len_utf16).sum::<usize>();
  let last = units(key).saturating_sub(1);
  let mut name = String::with_capacity(key.len());
  let mut unit = 0;
  for character in key.chars() {
    let next = unit + character.len_utf16();
    if next > 4 && unit < last {
      name.push(character);
    }
    unit = next;
  }
  name
}

/// Literal to raw value the way `convert_lit_to_raw_value` read SWC literals.
fn raw_of(value: &Value) -> Option<TRawValue> {
  match value {
    Value::Number(n) => Some(TRawValue::Number(*n)),
    Value::String(s) => Some(TRawValue::String(s.clone())),
    _ => None,
  }
}

fn insert_shifting(
  flattened: &mut IndexMap<String, PreRules>,
  first_order: &mut Option<&mut IndexSet<String>>,
  property: &str,
  pre_rule: PreRules,
) {
  if let Some(order) = first_order {
    order.insert(property.to_owned());
  }
  if flattened.contains_key(property) {
    flattened.shift_remove_entry(property);
  }
  flattened.insert(property.to_string(), pre_rule);
}

pub(crate) fn flatten_raw_style_object(core: &StyleXCore, style: &Object) -> IndexMap<String, PreRules> {
  let processed: Object = if core.options.enable_media_query_order {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
      last_media_query_wins_transform_value(style)
    })) {
      Ok(transformed) => transformed,
      Err(_) => stylex_panic!("{}", INVALID_MEDIA_QUERY_SYNTAX),
    }
  } else {
    style.clone()
  };
  let entries: Vec<(String, Value)> = processed.into_iter().collect();
  flatten_recording_order(core, &entries, &[], None)
}

fn flatten_recording_order(
  core: &StyleXCore,
  style: &[(String, Value)],
  key_path: &[String],
  mut first_order: Option<&mut IndexSet<String>>,
) -> IndexMap<String, PreRules> {
  let options = &core.options;
  let mut flattened: IndexMap<String, PreRules> = IndexMap::new();

  for (key, value) in style {
    let css_property_key: Cow<'_, str> = if CSS_VAR_REFERENCE.is_match(key).unwrap_or(false) {
      Cow::Owned(var_reference_name(key))
    } else {
      Cow::Borrowed(key.as_str())
    };

    match value {
      Value::Array(items) => {
        let mut equivalent_pairs: IndexMap<String, Vec<TRawValue>> = IndexMap::new();
        for item in items {
          match item {
            Value::String(_) | Value::Number(_) | Value::Null => {
              let pairs = flat_map_expanded_shorthands(
                (
                  Cow::Borrowed(css_property_key.as_ref()),
                  match raw_of(item) {
                    Some(val) => PreRuleValue::Raw(val),
                    None => PreRuleValue::Null,
                  },
                ),
                options,
              );
              for OrderPair(property, val) in pairs.iter() {
                let values = equivalent_pairs.entry(property.to_string()).or_default();
                if let Some(val) = val {
                  values.push(val.clone());
                }
              }
            },
            _ => stylex_panic!("{}", ILLEGAL_PROP_ARRAY_VALUE),
          }
        }

        for (property, values) in equivalent_pairs {
          let mut seen = FxHashSet::default();
          let mut values: Vec<TRawValue> = values
            .into_iter()
            .filter(|value| !value.is_falsy())
            .filter(|value| seen.insert(value.identity_key()))
            .collect();
          if values.is_empty() {
            insert_shifting(&mut flattened, &mut first_order, &property, PreRules::Null);
          } else {
            let pre_rule_value = match values.len() {
              1 => PreRuleValue::Raw(values.swap_remove(0)),
              _ => PreRuleValue::Vec(values),
            };
            let path = normalize_key_path(key_path.to_vec(), key, &property);
            insert_shifting(
              &mut flattened,
              &mut first_order,
              &property,
              PreRules::Styles(StylesPreRule::new(&property, pre_rule_value, path)),
            );
          }
        }
      },
      Value::String(_) | Value::Number(_) | Value::Null | Value::Bool(_) => {
        if !is_conditional_key(&css_property_key) {
          let pairs = flat_map_expanded_shorthands(
            (
              Cow::Borrowed(css_property_key.as_ref()),
              match raw_of(value) {
                Some(val) => PreRuleValue::Raw(val),
                None => PreRuleValue::Null,
              },
            ),
            options,
          );
          for OrderPair(property, pre_rule) in pairs.iter() {
            let property = property.to_string();
            let rule = match pre_rule {
              Some(pair_value) => {
                let path = normalize_key_path(key_path.to_vec(), &css_property_key, &property);
                PreRules::Styles(StylesPreRule::new(&property, PreRuleValue::Raw(pair_value.clone()), path))
              },
              None => PreRules::Null,
            };
            insert_shifting(&mut flattened, &mut first_order, &property, rule);
          }
        }
      },
      Value::Undefined => {
        stylex_macros::stylex_error::mark_panic_plain();
        stylex_panic!("{}", ILLEGAL_PROP_VALUE)
      },
      Value::Opaque(_) => stylex_panic!("{}", non_static_value("stylex")),
      Value::Object(obj) => {
        if !is_conditional_key(&css_property_key) {
          if obj.is_empty() {
            return flattened;
          }
          let mut equivalent_pairs: IndexMap<String, IndexMap<String, PreRules>> = IndexMap::new();
          for (condition, inner) in obj {
            let inner_entry = [(css_property_key.to_string(), inner.clone())];
            let path = if !key_path.is_empty() {
              let mut p = key_path.to_vec();
              p.push(condition.clone());
              p
            } else {
              vec![css_property_key.to_string(), condition.clone()]
            };
            let mut first_properties = IndexSet::new();
            let mut pairs =
              flatten_recording_order(core, &inner_entry, &path, Some(&mut first_properties));
            for property in first_properties {
              let pre_rule = pairs
                .swap_remove(&property)
                .expect("recorded flattened property has its final rule");
              equivalent_pairs
                .entry(property)
                .or_default()
                .insert(condition.clone(), pre_rule);
            }
          }
          for (property, by_condition) in equivalent_pairs {
            let rules: Vec<PreRules> = by_condition.into_values().collect();
            insert_shifting(&mut flattened, &mut first_order, &property, PreRules::create_set(rules));
          }
        } else {
          let inner: Vec<(String, Value)> = obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
          let mut path = key_path.to_vec();
          path.push(key.clone());
          let mut first_properties = IndexSet::new();
          let recording = first_order.is_some();
          let mut pairs = flatten_recording_order(
            core,
            &inner,
            &path,
            if recording { Some(&mut first_properties) } else { None },
          );
          let ordered: Vec<(String, PreRules)> = if recording {
            first_properties
              .into_iter()
              .map(|property| {
                let rule = pairs
                  .swap_remove(&property)
                  .expect("recorded prefixed property has its final rule");
                (property, rule)
              })
              .collect()
          } else {
            pairs.into_iter().collect()
          };
          for (property, rule) in ordered {
            insert_shifting(
              &mut flattened,
              &mut first_order,
              &format!("{css_property_key}_{property}"),
              rule,
            );
          }
        }
      },
    }
  }

  flattened
}

// ---------------------------------------------------------------------------------------------
// Dynamic styles
// ---------------------------------------------------------------------------------------------

/// One non-static leaf of a dynamic style function, replaced by a CSS variable.
#[derive(Debug, Clone, PartialEq)]
pub struct DynamicLeaf {
  /// The custom property the compiled class reads, e.g. `--x-color` or `--x-<hash>`.
  pub var_name: String,
  /// Key path from the namespace root to the leaf.
  pub path: Vec<String>,
  /// The front end's expression for the leaf.
  pub value: OpaqueId,
  /// Unit appended to numbers at runtime (`px`, `ms`...), `None` when the property is
  /// unitless. The front end emits `(v => typeof v === 'number' ? v + unit : v != null ? v :
  /// undefined)(expr)` with a unit, or `expr != null ? expr : undefined` without one.
  pub unit_suffix: Option<&'static str>,
}

/// `evaluate_partial_object_recursively`: replace every [`Value::Opaque`] leaf with
/// `var(--x-…)` and report it.
pub(crate) fn substitute_dynamic_leaves(
  body: &Object,
  key_path: &[String],
  leaves: &mut Vec<DynamicLeaf>,
) -> Object {
  let mut out = Object::with_capacity(body.len());
  for (raw_key, value) in body {
    let mut key = raw_key.clone();
    if key.starts_with("var(") && key.ends_with(')') && key_path.is_empty() {
      key = key[4..key.len() - 1].to_string();
    }
    match value {
      Value::Object(inner) => {
        let mut path = key_path.to_vec();
        path.push(key.clone());
        let replaced = substitute_dynamic_leaves(inner, &path, leaves);
        out.insert(key, Value::Object(replaced));
      },
      Value::Opaque(id) => {
        let mut full_path = key_path.to_vec();
        full_path.push(key.clone());
        let var_name = if !key_path.is_empty() {
          format!("--x-{}", create_hash(&full_path.join("_")))
        } else {
          format!("--x-{}", key)
        };
        out.insert(key.clone(), Value::String(format!("var({var_name})")));
        let prop_name = full_path
          .iter()
          .find(|k| !is_pseudo_selector(k) && !k.starts_with('@') && *k != "default")
          .unwrap_or(&key)
          .clone();
        let unit = if get_time_units().contains(prop_name.as_str())
          || LENGTH_UNITS.contains(prop_name.as_str())
        {
          get_number_suffix(prop_name.as_str())
        } else {
          ""
        };
        leaves.push(DynamicLeaf {
          var_name,
          path: full_path,
          value: *id,
          unit_suffix: (!unit.is_empty()).then_some(unit),
        });
      },
      other => {
        out.insert(key, other.clone());
      },
    }
  }
  out
}
