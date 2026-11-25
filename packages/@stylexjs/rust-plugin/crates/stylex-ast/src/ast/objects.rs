//! Operations on an object literal's own properties: enumeration order, duplicate keys and
//! spread merging, following JavaScript's rules.

use std::hash::BuildHasher;

use indexmap::IndexMap;
use rustc_hash::FxHashMap;
use stylex_utils::number::to_js_string;
use swc_core::atoms::Atom;
use swc_core::ecma::ast::{Prop, PropName, PropOrSpread};

use super::convertors::convert_wtf8_to_atom;

/// The name a property declares; `None` for spreads, accessors, methods and computed keys.
fn prop_key(prop: &PropOrSpread) -> Option<Atom> {
  match prop {
    PropOrSpread::Prop(prop) => match prop.as_ref() {
      Prop::Shorthand(ident) => Some(ident.sym.clone()),
      Prop::KeyValue(key_val) => match &key_val.key {
        PropName::Ident(ident) => Some(ident.sym.clone()),
        PropName::Str(strng) => Some(convert_wtf8_to_atom(&strng.value)),
        // `{ 42: x }` and `{ '42': x }` name one property; spelled via `to_js_string` (`1e+21`).
        PropName::Num(number) => Some(Atom::from(to_js_string(number.value))),
        // `{ 1n: x }` names the property `"1"`.
        PropName::BigInt(big_int) => Some(Atom::from(big_int.value.to_string())),
        _ => None,
      },
      _ => None,
    },
    _ => None,
  }
}

/// The array index a key spells: the canonical decimal form of an integer below 2^32 - 1
/// (`"0"` but not `"00"`, `"+0"` or `"01"`).
fn array_index_of(key: &str) -> Option<u32> {
  // Digits only, so `+0` stays a string key (Rust would accept the sign).
  if key.is_empty() || !key.bytes().all(|byte| byte.is_ascii_digit()) {
    return None;
  }

  if key.len() > 1 && key.starts_with('0') {
    return None;
  }

  let index = key.parse::<u32>().ok()?;

  match index == u32::MAX {
    true => None,
    false => Some(index),
  }
}

/// Stable-sort key for JavaScript enumeration order: array indices first by value, then the
/// rest in insertion order.
fn own_key_rank(name: Option<&str>) -> (bool, u32) {
  match name.and_then(array_index_of) {
    Some(index) => (false, index),
    None => (true, 0),
  }
}

/// Whether a name is an array index; checked first to skip sorting in the common case.
fn is_index_name(name: Option<&str>) -> bool {
  name.and_then(array_index_of).is_some()
}

/// The array index a number key spells, without allocating its string form.
fn number_array_index(value: f64) -> Option<u32> {
  let is_index = value >= 0.0 && value < f64::from(u32::MAX) && value.fract() == 0.0;

  is_index.then_some(value as u32)
}

/// The array index a property's key spells; must agree with `array_index_of(prop_key(..))`.
fn prop_array_index(prop: &PropOrSpread) -> Option<u32> {
  let PropOrSpread::Prop(prop) = prop else {
    return None;
  };

  match prop.as_ref() {
    Prop::Shorthand(ident) => array_index_of(&ident.sym),
    Prop::KeyValue(key_val) => match &key_val.key {
      PropName::Ident(ident) => array_index_of(&ident.sym),
      // Same convertor as `prop_key`, so unspellable text is refused the same way.
      PropName::Str(strng) => array_index_of(&convert_wtf8_to_atom(&strng.value)),
      PropName::Num(number) => number_array_index(number.value),
      PropName::BigInt(big_int) => array_index_of(&big_int.value.to_string()),
      _ => None,
    },
    _ => None,
  }
}

/// Reorders properties into JavaScript enumeration order: array-index keys ascending, then the
/// rest in insertion order. This decides stylesheet order, so it must match upstream.
pub fn order_own_keys(props: Vec<PropOrSpread>) -> Vec<PropOrSpread> {
  let mut indexed: Vec<(u32, PropOrSpread)> = Vec::new();
  let mut named: Vec<PropOrSpread> = Vec::with_capacity(props.len());

  for prop in props {
    match prop_array_index(&prop) {
      Some(index) => indexed.push((index, prop)),
      None => named.push(prop),
    }
  }

  if indexed.is_empty() {
    return named;
  }

  indexed.sort_by_key(|(index, _)| *index);

  let mut ordered = Vec::with_capacity(indexed.len() + named.len());
  ordered.extend(indexed.into_iter().map(|(_, prop)| prop));
  ordered.extend(named);

  ordered
}

/// [`order_own_keys`] for an ordered map; `name_of` returning `None` ranks as a string key.
pub fn order_own_map_keys<K, V, S>(
  map: &mut IndexMap<K, V, S>,
  name_of: impl Fn(&K) -> Option<&str>,
) where
  S: BuildHasher,
{
  if !map.keys().any(|key| is_index_name(name_of(key))) {
    return;
  }

  map.sort_by_cached_key(|key, _| own_key_rank(name_of(key)));
}

/// Keeps the last property under each name, in the position the name first took, and drops
/// unnamed properties (like `Object.entries({ color: 1, opacity: 2, color: 3 })`).
pub fn remove_duplicates(props: Vec<PropOrSpread>) -> Vec<PropOrSpread> {
  let mut positions: FxHashMap<Atom, usize> = FxHashMap::default();
  let mut result: Vec<PropOrSpread> = Vec::with_capacity(props.len());

  for prop in props {
    let Some(key) = prop_key(&prop) else {
      continue;
    };

    match positions.get(&key) {
      Some(&position) => result[position] = prop,
      None => {
        positions.insert(key, result.len());
        result.push(prop);
      },
    }
  }

  result
}

/// The properties of `{ ...old_props, ...new_props }`: a shallow `Object.assign`, where a
/// repeated key takes the later value at the position it first took.
pub fn assign_props(
  old_props: Vec<PropOrSpread>,
  new_props: Vec<PropOrSpread>,
) -> Vec<PropOrSpread> {
  let mut merged: Vec<PropOrSpread> = Vec::with_capacity(old_props.len() + new_props.len());
  let mut positions: FxHashMap<Atom, usize> = FxHashMap::default();

  for prop in old_props.into_iter().chain(new_props) {
    let Some(key) = prop_key(&prop) else {
      // Unnamed properties keep their place and collide with nothing.
      merged.push(prop);
      continue;
    };

    match positions.get(&key) {
      Some(&position) => merged[position] = prop,
      None => {
        positions.insert(key, merged.len());
        merged.push(prop);
      },
    }
  }

  merged
}
