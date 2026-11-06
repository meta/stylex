use std::borrow::Cow;

use indexmap::IndexMap;

use stylex_macros::stylex_panic;
use stylex_structures::pre_rule_value::PreRuleValue;
use swc_core::ecma::ast::{Expr, KeyValueProp};

use crate::shared::utils::core::flat_map_expanded_shorthands::flat_map_expanded_shorthands;
use stylex_ast::ast::convertors::{convert_key_value_to_str, get_key_values_from_object};
use stylex_state::state_manager::StateManager;
use stylex_structures::{order_pair::OrderPair, pair::Pair, raw_value::TRawValue};

use super::css::common::transform_value_cached;

pub(crate) fn obj_entries(obj: &Expr) -> Vec<KeyValueProp> {
  let object = match obj.as_object() {
    Some(o) => o,
    None => stylex_panic!("Object expected"),
  };

  get_key_values_from_object(object)
}

pub(crate) fn obj_from_entries(entries: &[OrderPair]) -> IndexMap<String, TRawValue> {
  let mut map = IndexMap::with_capacity(entries.len());

  for OrderPair(key, value) in entries {
    map.insert(
      key.to_string(),
      match value.as_ref() {
        Some(v) => v.clone(),
        None => stylex_panic!("Value is not a string"),
      },
    );
  }

  map
}

/// Maps each key and converts each raw value to its final CSS text.
///
/// The two steps are fused because their order is observable: the key is
/// dashified before the value is transformed, so the property name that decides
/// the unit suffix is the dashed one.
///
/// Two keys that map to the same name are one declaration, and the value
/// written last wins, so the pairs are collected through a map before they are
/// answered.
pub(crate) fn obj_map_keys_and_transform_values(
  entries: &IndexMap<String, TRawValue>,
  state: &mut StateManager,
  mapper: impl Fn(&str) -> String,
) -> Vec<Pair> {
  let mut map: IndexMap<String, String> = IndexMap::with_capacity(entries.len());

  for (key, value) in entries {
    let mapped_key = mapper(key);
    let value = transform_value_cached(mapped_key.as_str(), value, state);

    map.insert(mapped_key, value);
  }

  map
    .into_iter()
    .map(|(key, value)| Pair { key, value })
    .collect()
}

pub(crate) fn preprocess_object_properties(
  style: &Expr,
  state: &mut StateManager,
) -> IndexMap<String, TRawValue> {
  let res: Vec<OrderPair> = obj_entries(style)
    .iter()
    .flat_map(|pair| {
      let key = convert_key_value_to_str(pair);

      flat_map_expanded_shorthands(
        (Cow::Owned(key), PreRuleValue::Expr(*pair.value.clone())),
        &state.options,
      )
      .into_iter()
      .collect::<Vec<OrderPair>>()
    })
    .filter(|item| item.1.is_some())
    .collect::<Vec<OrderPair>>();

  obj_from_entries(&res)
}

#[cfg(test)]
#[path = "tests/object_tests.rs"]
mod tests;
