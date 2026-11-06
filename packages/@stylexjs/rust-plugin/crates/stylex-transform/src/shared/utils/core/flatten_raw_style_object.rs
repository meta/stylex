use std::borrow::Cow;

use indexmap::IndexMap;
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

/// The key path, with the authored key replaced by the property it expanded to.
///
/// A path that does not already name the key gains the property at its end.
/// The key is compared as text: asking `contains` for it made a string of it
/// once per property, only to throw it away.
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

/// `tpl`, or the refusal a handled template that is not one is reported with.
///
/// This is the whole of what is left out of the coverage measurement, and it
/// computes nothing -- it chooses between answers the caller has already worked
/// out. The handler is given a template literal and gives one back.
/// `guidelines/stack/RUST.md` describes the allowance.
#[cfg_attr(coverage_nightly, coverage(off))]
fn or_refuse_handled_template(tpl: Option<&Tpl>) -> &Tpl {
  match tpl {
    Some(tpl) => tpl,
    None => stylex_panic!("Expected a template literal expression."),
  }
}

/// Reports a key the variable-reference expression gave up on, and reads it as
/// no match.
///
/// This is the whole of what is left out of the coverage measurement. It does
/// two things -- it writes the warning and it answers `false` -- and neither is
/// reachable from a source: the expression answers an error only when the
/// matcher gives up on a backtrack, which no key can ask it to do. A `false`
/// answered here is the same `false` the matcher answers for a key that holds
/// no reference, so nothing downstream tells the two apart.
/// `guidelines/stack/RUST.md` describes the allowance.
#[cfg_attr(coverage_nightly, coverage(off))]
fn report_unmatched_key(error: impl std::fmt::Display) -> bool {
  warn!("Error matching CSS_VAR_REFERENCE: {error}. Skipping pattern match.");

  false
}

/// Names a key by what its brackets wrap: the key without its first four units
/// and its last one, which is what `var(` and `)` spell. The cut is a fixed one
/// rather than the span the expression matched, because the expression is asked
/// of the whole key -- so a key that carries the reference in its middle is
/// named by the text around it.
///
/// A unit is what JavaScript counts, so the name agrees with the reference
/// implementation for every key both can spell. Counting bytes instead stopped
/// the build on a key carrying text outside ASCII, and counting characters
/// named such a key differently, because JavaScript spells some characters
/// with two units.
///
/// A character the cut falls inside is kept whole. JavaScript gives one half
/// of it, which is no text this compiler can carry.
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
  let mut flattened: IndexMap<String, PreRules> = IndexMap::new();

  for property in style.iter() {
    let key = convert_key_value_to_str(property);

    // A key holding a `var(--name)` reference is named by what the brackets
    // wrap, which is how a variable set in a `create` call reaches the custom
    // property it declares. Every other key keeps the name the author wrote.
    //
    // The expression is tested against the whole key, so a reference written
    // inside a longer key is named by the text around it.
    //
    // The shorthand expansion in `flat_map_expanded_shorthands.rs` asks a
    // second, wider question of the same key, and a key this one passes over is
    // named there. The two are not one question written twice.
    //
    // Borrowed rather than copied. The conditions of one property are each
    // named after it, and every name was a string of its own.
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
        // Step 1: Expand properties to its constituent parts
        // Collect the various values for each value in the array
        // that belongs to the same property.

        let mut equivalent_pairs: IndexMap<String, Vec<TRawValue>> = IndexMap::new();

        property_array.elems.iter().for_each(|each_val| {
          if let Some(property) = each_val {
            match property.expr.as_ref() {
              // `null` is the one element that contributes nothing without
              // making the array itself wrong: it carries no value, and the
              // entries around it keep their order and their place in the
              // chain.
              //
              // It still expands, and the properties it expands to are still
              // registered -- with nothing pushed onto them. That is what makes
              // an array holding only `null` declare an absent value for each
              // of those properties instead of vanishing: the entry exists with
              // an empty value list, and the loop below turns exactly that into
              // a `NullPreRule`. Skipping the expansion here would leave no
              // entry at all, and the property would be missing from the style
              // object rather than present and unset.
              //
              // A string and a number are the only values a declaration in a
              // fallback chain can carry. Every other literal is refused by the
              // value validator before reaching here; the arm below is what
              // holds if a caller ever reaches this function without it.
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
                  // Named once, where the property is new. The values of one
                  // property arrive one after another, so asking `entry` for it
                  // made a string of a name the map already held.
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

            insert_or_update_rule_with_shifting_index(&mut flattened, &property, pre_rule);
          } else {
            // At least one value, because the empty set is the branch above.
            // Taken out of the set rather than copied out of it: the set is
            // owned by this turn of the loop and is read no further, so a copy
            // of every value of the declaration went straight to waste.
            let pre_rule_value = match values.len() {
              1 => PreRuleValue::Raw(values.swap_remove(0)),
              _ => PreRuleValue::Vec(values),
            };

            // The authored key, which is the one a fallback list is named
            // after. The reference implementation asks the two arms this
            // question differently, and both spellings are kept.
            let normalized_key_path =
              normalize_key_path(key_path.clone(), key.as_str(), property.as_str());

            let pre_rule = PreRules::StylesPreRule(StylesPreRule::new(
              property.as_str(),
              pre_rule_value,
              Some(normalized_key_path),
            ));
            insert_or_update_rule_with_shifting_index(&mut flattened, &property, pre_rule);
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

              insert_or_update_rule_with_shifting_index(&mut flattened, &property, pre_rule);
            } else {
              let pre_rule = PreRules::NullPreRule(NullPreRule::new());
              insert_or_update_rule_with_shifting_index(&mut flattened, &property, pre_rule);
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

        // The key names the property here, so the path gains it where it does
        // not already hold it.
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

        flattened.insert(css_property_key.into_owned(), pre_rule);
      },
      Expr::Ident(ident) => match get_var_decl_by_ident(ident, traversal_state, fns) {
        Some(var_decl) => {
          let var_decl_expr = get_expr_from_var_decl(&var_decl);

          let mut property_cloned = property.clone();
          property_cloned.value = Box::new(var_decl_expr.clone());

          let inner_flattened = flatten_raw_style_object_logic(
            &[property_cloned],
            key_path,
            state,
            traversal_state,
            fns,
          );

          flattened.extend(inner_flattened);
        },
        // `undefined` is a value this evaluator is confident about rather than
        // a name it failed to resolve: it is what a key an object does not
        // carry, an index past the end of an array, and a member read off a
        // folded function map all answer. A style value position refuses it
        // for not being a style value, which is the sentence the reference
        // implementation gives it -- where saying nothing is static reads as a
        // report about resolution, and names neither the value nor the input.
        _ if is_js_undefined(ident) => stylex_panic!("{}", ILLEGAL_PROP_VALUE),
        _ => {
          stylex_panic!("{}", non_static_value("stylex"));
        },
      },
      Expr::Bin(bin) => {
        let result = transform_bin_expr_to_number(bin, state, traversal_state, fns);

        let mut property_cloned = property.clone();
        property_cloned.value = Box::new(create_number_expr(result));

        let inner_flattened =
          flatten_raw_style_object_logic(&[property_cloned], key_path, state, traversal_state, fns);

        flattened.extend(inner_flattened)
      },
      Expr::Call(_) => stylex_panic!("{}", non_static_value("stylex")),
      Expr::Object(obj) => {
        if !is_conditional_key(&css_property_key) {
          if obj.props.is_empty() {
            return flattened;
          }
          let mut equivalent_pairs: IndexMap<String, IndexMap<String, PreRules>> = IndexMap::new();

          // One copy of each condition, made where it is expanded. Copying the
          // whole object first and then the condition out of the copy made two.
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

                let pairs = flatten_raw_style_object_logic(
                  std::slice::from_ref(inner_key_value),
                  &mut key_path,
                  state,
                  traversal_state,
                  fns,
                );

                for (property, pre_rule) in pairs {
                  // One look-up rather than a look-up, an insert and a second
                  // look-up. The entry is the map either way, so the refusal
                  // that stood here answered for nothing.
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

            flattened.insert(property.clone(), PreRuleSet::create(rules));
          }
        } else {
          let inner_key_value = get_key_values_from_object(obj);

          let mut key_path = key_path.clone();
          key_path.push(key.clone());

          let pairs = flatten_raw_style_object_logic(
            &inner_key_value.into_iter().collect::<Vec<KeyValueProp>>(),
            &mut key_path,
            state,
            traversal_state,
            fns,
          );

          for (property, pre_rule) in pairs {
            insert_or_update_rule_with_shifting_index(
              &mut flattened,
              format!("{}_{}", css_property_key, property).as_str(),
              pre_rule,
            );
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

fn insert_or_update_rule_with_shifting_index(
  flattened: &mut IndexMap<String, PreRules>,
  property: &str,
  pre_rule: PreRules,
) {
  if flattened.get(property).is_some() {
    flattened.shift_remove_entry(property);
  };

  flattened.insert(property.to_string(), pre_rule);
}

#[cfg(test)]
#[path = "tests/flatten_raw_style_object_tests.rs"]
mod tests;
