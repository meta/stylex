use std::borrow::Cow;

use indexmap::{IndexMap, IndexSet};
use log::warn;
use rustc_hash::FxHashSet;
use stylex_css_parser::at_queries::media_query_transform::last_media_query_wins_transform;
use stylex_macros::stylex_panic;
use stylex_structures::pre_rule_value::PreRuleValue;
use swc_core::ecma::{
  ast::{Expr, KeyValueProp, Lit, Prop, PropName, PropOrSpread, Tpl},
  utils::quote_str,
};

use crate::shared::structures::{
  null_pre_rule::NullPreRule,
  pre_rule::{PreRules, StylesPreRule},
  pre_rule_set::PreRuleSet,
};
use stylex_ast::ast::convertors::{
  convert_key_value_to_str, create_number_expr, expand_shorthand_prop, get_expr_from_var_decl,
  get_key_values_from_object, is_js_undefined,
};
use stylex_constants::constants::messages::{
  ILLEGAL_PROP_ARRAY_VALUE, ILLEGAL_PROP_VALUE, INVALID_MEDIA_QUERY_SYNTAX, non_static_value,
};
use stylex_css::utils::condition::is_conditional_key;
use stylex_evaluator::convertors::{expr_tpl_to_string, transform_bin_expr_to_number};
use stylex_evaluator::state::EvaluationState;
use stylex_regex::regex::CSS_VAR_REFERENCE;
use stylex_state::resolution::convertors::{convert_lit_to_raw_value, handle_tpl_to_expression};
use stylex_state::resolution::lookup::get_var_decl_by_ident;
use stylex_state::{functions::FunctionMap, state_manager::StateManager};
use stylex_structures::{order_pair::OrderPair, raw_value::TRawValue};

use super::flat_map_expanded_shorthands::flat_map_expanded_shorthands;

/// The key path, with the property appended when the path doesn't already name the key.
fn normalize_key_path(key_path: Vec<String>, key: &str, property: &str) -> Vec<String> {
  if key_path.iter().any(|step| step == key) {
    key_path
      .into_iter()
      .map(|step| {
        if step == key {
          property.to_string()
        } else {
          step
        }
      })
      .collect()
  } else {
    let mut new_key_path = key_path;

    new_key_path.push(property.to_string());

    new_key_path
  }
}

pub(crate) fn flatten_raw_style_object(
  style: &[KeyValueProp],
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> IndexMap<String, PreRules> {
  let mut processed_style = style.to_vec();

  if traversal_state.options.enable_media_query_order {
    let transform_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
      last_media_query_wins_transform(style)
    }));

    match transform_result {
      Ok(transformed) => {
        processed_style = transformed;
      },
      Err(_) => {
        stylex_panic!("{}", INVALID_MEDIA_QUERY_SYNTAX);
      },
    }
  }

  flatten_raw_style_object_logic(&processed_style, &mut vec![], state, traversal_state, fns)
}

/// `tpl`, or the refusal for a handled non-template (unreachable; excluded from coverage).
#[cfg_attr(coverage_nightly, coverage(off))]
fn or_refuse_handled_template(tpl: Option<&Tpl>) -> &Tpl {
  match tpl {
    Some(tpl) => tpl,
    None => stylex_panic!("Expected a template literal expression."),
  }
}

/// Warns about a key the variable-reference regex gave up on and treats it as no match
/// (unreachable; excluded from coverage).
#[cfg_attr(coverage_nightly, coverage(off))]
fn report_unmatched_key(error: impl std::fmt::Display) -> bool {
  warn!("Error matching CSS_VAR_REFERENCE: {error}. Skipping pattern match.");

  false
}

/// The key minus its first four and last UTF-16 units (`var(` and `)`), as JavaScript slices it.
/// A character split by the cut is kept whole.
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

pub(crate) fn flatten_raw_style_object_logic(
  style: &[KeyValueProp],
  key_path: &mut Vec<String>,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> IndexMap<String, PreRules> {
  flatten_raw_style_object_recording_order(style, key_path, state, traversal_state, fns, None)
}

// Babel's inner flatten keeps duplicates; most callers need only the keep-last map. Also
// records first appearance for conditional-property aggregation.
fn flatten_raw_style_object_recording_order(
  style: &[KeyValueProp],
  key_path: &mut Vec<String>,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
  mut first_order: Option<&mut IndexSet<String>>,
) -> IndexMap<String, PreRules> {
  let mut flattened: IndexMap<String, PreRules> = IndexMap::new();

  for property in style.iter() {
    let key = convert_key_value_to_str(property);

    // A `var(--name)` key is named by what the brackets wrap; others keep the authored name.
    // `flat_map_expanded_shorthands.rs` asks a second, wider question of the same key.
    let css_property_key: Cow<'_, str> = if CSS_VAR_REFERENCE
      .is_match(&key)
      .unwrap_or_else(report_unmatched_key)
    {
      Cow::Owned(var_reference_name(&key))
    } else {
      Cow::Borrowed(key.as_str())
    };

    match property.value.as_ref() {
      Expr::Array(property_array) => {
        // Expand each array value and collect values per property.

        let mut equivalent_pairs: IndexMap<String, Vec<TRawValue>> = IndexMap::new();

        property_array.elems.iter().for_each(|each_val| {
          if let Some(property) = each_val {
            match property.expr.as_ref() {
              // `null` adds no value but still registers its properties, so an all-`null`
              // array yields a `NullPreRule` per property. Validators refuse other literals.
              Expr::Lit(property_lit @ (Lit::Str(_) | Lit::Num(_) | Lit::Null(_))) => {
                let pairs = flat_map_expanded_shorthands(
                  (
                    Cow::Borrowed(css_property_key.as_ref()),
                    match convert_lit_to_raw_value(property_lit) {
                      Some(val) => PreRuleValue::Raw(val),
                      None => PreRuleValue::Null,
                    },
                  ),
                  &traversal_state.options,
                );

                for OrderPair(property, val) in pairs.iter() {
                  let values = match equivalent_pairs.get_mut(property.as_ref()) {
                    Some(values) => values,
                    None => equivalent_pairs.entry(property.to_string()).or_default(),
                  };

                  if let Some(val) = val {
                    values.push(val.clone());
                  }
                }
              },
              _ => stylex_panic!("{}", ILLEGAL_PROP_ARRAY_VALUE),
            }
          }
        });

        for (property, values) in equivalent_pairs {
          // Drop falsy values, then deduplicate by JS identity: every repeat is
          // removed, not just adjacent ones, and `0` and `"0"` stay distinct.
          let mut seen = FxHashSet::default();
          let mut values = values
            .into_iter()
            .filter(|value| !value.is_falsy())
            .filter(|value| seen.insert(value.identity_key()))
            .collect::<Vec<TRawValue>>();

          if values.is_empty() {
            let pre_rule = PreRules::NullPreRule(NullPreRule::new());

            insert_or_update_rule_with_shifting_index(
              &mut flattened,
              &mut first_order,
              &property,
              pre_rule,
            );
          } else {
            // Non-empty: the empty case is the branch above.
            let pre_rule_value = match values.len() {
              1 => PreRuleValue::Raw(values.swap_remove(0)),
              _ => PreRuleValue::Vec(values),
            };

            // Fallback lists are named after the authored key, as in the reference.
            let normalized_key_path =
              normalize_key_path(key_path.clone(), key.as_str(), property.as_str());

            let pre_rule = PreRules::StylesPreRule(StylesPreRule::new(
              property.as_str(),
              pre_rule_value,
              Some(normalized_key_path),
            ));
            insert_or_update_rule_with_shifting_index(
              &mut flattened,
              &mut first_order,
              &property,
              pre_rule,
            );
          }
        }
      },
      Expr::Lit(property_lit) => {
        if !is_conditional_key(&css_property_key) {
          let value = convert_lit_to_raw_value(property_lit);

          let pairs = flat_map_expanded_shorthands(
            (
              Cow::Borrowed(css_property_key.as_ref()),
              match value {
                Some(val) => PreRuleValue::Raw(val),
                None => PreRuleValue::Null,
              },
            ),
            &traversal_state.options,
          );

          for OrderPair(property, pre_rule) in pairs.iter() {
            let property = property.to_string();

            if let Some(pair_value) = pre_rule {
              let normalized_key_path = normalize_key_path(
                key_path.clone(),
                css_property_key.as_ref(),
                property.as_str(),
              );

              let pre_rule = PreRules::StylesPreRule(StylesPreRule::new(
                property.as_str(),
                PreRuleValue::Raw(pair_value.clone()),
                Some(normalized_key_path),
              ));

              insert_or_update_rule_with_shifting_index(
                &mut flattened,
                &mut first_order,
                &property,
                pre_rule,
              );
            } else {
              let pre_rule = PreRules::NullPreRule(NullPreRule::new());
              insert_or_update_rule_with_shifting_index(
                &mut flattened,
                &mut first_order,
                &property,
                pre_rule,
              );
            }
          }
        }
      },
      Expr::Tpl(tpl) => {
        let handled_tpl = handle_tpl_to_expression(tpl, traversal_state, fns);
        let result = expr_tpl_to_string(
          or_refuse_handled_template(handled_tpl.as_tpl()),
          state,
          traversal_state,
          fns,
        );

        let normalized_key_path = normalize_key_path(
          key_path.clone(),
          css_property_key.as_ref(),
          css_property_key.as_ref(),
        );

        let pre_rule = PreRules::StylesPreRule(StylesPreRule::new(
          css_property_key.as_ref(),
          PreRuleValue::string(result),
          Some(normalized_key_path),
        ));

        if let Some(order) = &mut first_order {
          order.insert(css_property_key.to_string());
        }
        flattened.insert(css_property_key.into_owned(), pre_rule);
      },
      Expr::Ident(ident) => match get_var_decl_by_ident(ident, traversal_state, fns) {
        Some(var_decl) => {
          let var_decl_expr = get_expr_from_var_decl(&var_decl);

          let mut property_cloned = property.clone();
          property_cloned.value = Box::new(var_decl_expr.clone());

          let inner_flattened = flatten_raw_style_object_recording_order(
            &[property_cloned],
            key_path,
            state,
            traversal_state,
            fns,
            first_order.as_deref_mut(),
          );

          flattened.extend(inner_flattened);
        },
        // `undefined` is a confident value: refuse it as a non-style value, as the reference does.
        _ if is_js_undefined(ident) => {
          stylex_macros::stylex_error::mark_panic_plain();
          stylex_panic!("{}", ILLEGAL_PROP_VALUE)
        },
        _ => {
          stylex_panic!("{}", non_static_value("stylex"));
        },
      },
      Expr::Bin(bin) => {
        let result = transform_bin_expr_to_number(bin, state, traversal_state, fns);

        let mut property_cloned = property.clone();
        property_cloned.value = Box::new(create_number_expr(result));

        let inner_flattened = flatten_raw_style_object_recording_order(
          &[property_cloned],
          key_path,
          state,
          traversal_state,
          fns,
          first_order.as_deref_mut(),
        );

        flattened.extend(inner_flattened)
      },
      Expr::Call(_) => stylex_panic!("{}", non_static_value("stylex")),
      Expr::Object(obj) => {
        if !is_conditional_key(&css_property_key) {
          if obj.props.is_empty() {
            return flattened;
          }
          let mut equivalent_pairs: IndexMap<String, IndexMap<String, PreRules>> = IndexMap::new();

          for prop in &obj.props {
            if let PropOrSpread::Prop(prop) = prop {
              let mut prop = prop.clone();

              expand_shorthand_prop(&mut prop);

              if let Prop::KeyValue(inner_key_value) = prop.as_mut() {
                let condition = convert_key_value_to_str(inner_key_value);

                inner_key_value.key = PropName::Str(quote_str!(css_property_key.as_ref()));

                let mut key_path = if !key_path.is_empty() {
                  let mut new_key_path = key_path.clone();
                  new_key_path.push(condition.clone());
                  new_key_path
                } else {
                  vec![css_property_key.to_string(), condition.clone()]
                };

                let mut first_properties = IndexSet::new();
                let mut pairs = flatten_raw_style_object_recording_order(
                  std::slice::from_ref(inner_key_value),
                  &mut key_path,
                  state,
                  traversal_state,
                  fns,
                  Some(&mut first_properties),
                );

                // Aggregate by each property's first appearance in Babel's appended list.
                for property in first_properties {
                  let pre_rule = pairs
                    .swap_remove(&property)
                    .expect("recorded flattened property has its final rule");
                  equivalent_pairs
                    .entry(property)
                    .or_default()
                    .insert(condition.clone(), pre_rule);
                }
              } else {
                stylex_panic!("{}", non_static_value("stylex"));
              }
            } else {
              stylex_panic!("{}", non_static_value("stylex"));
            }
          }

          for (property, obj) in equivalent_pairs.iter() {
            let sorted_keys: Vec<&String> = obj.keys().collect();

            let rules: Vec<PreRules> = sorted_keys
              .iter()
              .map(|condition| obj[*condition].clone())
              .collect();

            // Babel appends this pair, then removes earlier duplicates while
            // retaining the last slot (2d42d72 flatten-raw-style-obj.js:158-166;
            // stylex-create.js:59-74).
            insert_or_update_rule_with_shifting_index(
              &mut flattened,
              &mut first_order,
              property,
              PreRuleSet::create(rules),
            );
          }
        } else {
          let inner_key_value = get_key_values_from_object(obj);

          let mut key_path = key_path.clone();
          key_path.push(key.clone());

          let mut first_properties = IndexSet::new();
          let mut pairs = flatten_raw_style_object_recording_order(
            &inner_key_value.into_iter().collect::<Vec<KeyValueProp>>(),
            &mut key_path,
            state,
            traversal_state,
            fns,
            first_order.as_ref().map(|_| &mut first_properties),
          );

          if first_order.is_some() {
            let pairs = first_properties.into_iter().map(|property| {
              let rule = pairs
                .swap_remove(&property)
                .expect("recorded prefixed property has its final rule");
              (property, rule)
            });
            insert_prefixed_rules(&mut flattened, &mut first_order, &css_property_key, pairs);
          } else {
            insert_prefixed_rules(&mut flattened, &mut first_order, &css_property_key, pairs);
          }
        }
      },
      _ => {
        stylex_panic!("{}", ILLEGAL_PROP_VALUE)
      },
    };
  }

  flattened
}

fn insert_prefixed_rules(
  flattened: &mut IndexMap<String, PreRules>,
  first_order: &mut Option<&mut IndexSet<String>>,
  prefix: &str,
  pairs: impl IntoIterator<Item = (String, PreRules)>,
) {
  for (property, rule) in pairs {
    insert_or_update_rule_with_shifting_index(
      flattened,
      first_order,
      &format!("{prefix}_{property}"),
      rule,
    );
  }
}

fn insert_or_update_rule_with_shifting_index(
  flattened: &mut IndexMap<String, PreRules>,
  first_order: &mut Option<&mut IndexSet<String>>,
  property: &str,
  pre_rule: PreRules,
) {
  if let Some(order) = first_order {
    order.insert(property.to_owned());
  }
  if flattened.get(property).is_some() {
    flattened.shift_remove_entry(property);
  };

  flattened.insert(property.to_string(), pre_rule);
}

#[cfg(test)]
#[path = "tests/flatten_raw_style_object_tests.rs"]
mod tests;
