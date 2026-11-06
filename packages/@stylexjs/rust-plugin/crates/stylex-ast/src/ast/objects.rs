//! Operations on the own properties of an object literal.
//!
//! An object literal is a list of properties, and three questions get asked of
//! that list often enough to be answered once: what order does the language
//! enumerate it in, which properties does a later one replace, and what does
//! spreading one object into another produce. Every answer turns on which key a
//! property declares, so that reading is stated once here too.
//!
//! The first question is also asked of collections that are not property lists.
//! A `create` call collects its namespaces into an ordered map, and that map
//! stands for an object too, so it enumerates by the same rule. Both readers
//! share `own_key_rank` rather than each carrying an opinion about `+0`.

use std::hash::BuildHasher;

use indexmap::IndexMap;
use rustc_hash::FxHashMap;
use stylex_utils::number::to_js_string;
use swc_core::atoms::Atom;
use swc_core::ecma::ast::{Prop, PropName, PropOrSpread};

use super::convertors::convert_wtf8_to_atom;

/// The name a property declares, or `None` where it declares no nameable one.
///
/// `None` is a spread, a getter, a setter, a method, or a computed key -- each
/// of which either has no key at all or has one this cannot read without
/// evaluating it. Stated once because three readers depend on the answer, and
/// three spellings of "which key is this" would let them disagree about whether
/// two properties collide.
fn prop_key(prop: &PropOrSpread) -> Option<Atom> {
  match prop {
    PropOrSpread::Prop(prop) => match prop.as_ref() {
      Prop::Shorthand(ident) => Some(ident.sym.clone()),
      Prop::KeyValue(key_val) => match &key_val.key {
        PropName::Ident(ident) => Some(ident.sym.clone()),
        PropName::Str(strng) => Some(convert_wtf8_to_atom(&strng.value)),
        // A numeric key is a string key spelled as a number: `{ 42: x }` and
        // `{ '42': x }` name one property in the language, so they have to read
        // as one name here. Rendered through `to_js_string` so the spelling is
        // the language's -- `1e+21`, not `1000000000000000000000`.
        PropName::Num(number) => Some(Atom::from(to_js_string(number.value))),
        // `{ 1n: x }` names the property `"1"`, as every non-computed key that
        // is not already a string does.
        PropName::BigInt(big_int) => Some(Atom::from(big_int.value.to_string())),
        _ => None,
      },
      _ => None,
    },
    _ => None,
  }
}

/// Whether a key is an array index, in the sense JavaScript uses to decide
/// enumeration order: the canonical decimal spelling of an integer below
/// 2^32 - 1.
///
/// Canonical is what makes `"0"` one and `"00"`, `"+0"` and `"01"` not — those
/// round-trip to a different string, so the language treats them as ordinary
/// string keys and enumerates them in insertion order.
fn array_index_of(key: &str) -> Option<u32> {
  // Only digits, so a signed spelling such as `+0` stays a string key -- Rust
  // reads the sign that JavaScript does not. This also settles every ordinary
  // property name at its first byte, before any parse runs.
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

/// Where a name sorts among the own keys of an object, as a key a stable sort
/// can order by.
///
/// The whole enumeration rule lives here: an array-index name sorts before
/// every other name, two of them sort by value, and every other name gets the
/// same rank so a stable sort leaves them in the order they were written. The
/// leading flag carries the first half -- `false` sorts before `true` -- and the
/// index carries the second.
///
/// Used by the map reader, whose container cannot be split in place the way a
/// property list can.
fn own_key_rank(name: Option<&str>) -> (bool, u32) {
  match name.and_then(array_index_of) {
    Some(index) => (false, index),
    None => (true, 0),
  }
}

/// Whether a name is one of those that move to the front.
///
/// Asked once per name before any sort runs. An object with no array-index name
/// already enumerates in the order it was written, which is nearly every object
/// this compiler reads, and that pass keeps the common case free of a sort and
/// of the storage a sort needs. `array_index_of` rejects an ordinary name at its
/// first byte, so the pass costs one byte comparison per name.
fn is_index_name(name: Option<&str>) -> bool {
  name.and_then(array_index_of).is_some()
}

/// The index a number key spells, without spelling it.
///
/// `prop_key` renders a number key through `to_js_string`, which allocates. The
/// ordering only asks whether the name is an array index, and that is a question
/// about the number itself: the language spells a whole number below 2^32 as its
/// digits and nothing else, so the two readings agree without the string. An
/// exponent form starts at 1e21, well above the range, and `-0` spells `0` in
/// the language exactly as the comparison below reads it.
fn number_array_index(value: f64) -> Option<u32> {
  let is_index = value >= 0.0 && value < f64::from(u32::MAX) && value.fract() == 0.0;

  is_index.then_some(value as u32)
}

/// The array index a property's key spells, or `None` where it spells none.
///
/// Reads the key where it stands rather than through `prop_key`, which builds an
/// owned name this would only measure and drop. Every arm has to agree with
/// `array_index_of(prop_key(..))`, which the tests hold it to.
fn prop_array_index(prop: &PropOrSpread) -> Option<u32> {
  let PropOrSpread::Prop(prop) = prop else {
    return None;
  };

  match prop.as_ref() {
    Prop::Shorthand(ident) => array_index_of(&ident.sym),
    Prop::KeyValue(key_val) => match &key_val.key {
      PropName::Ident(ident) => array_index_of(&ident.sym),
      // Through the same convertor `prop_key` uses, so text no `str` can spell
      // is refused here the way it is refused there rather than read as "not an
      // index".
      PropName::Str(strng) => array_index_of(&convert_wtf8_to_atom(&strng.value)),
      PropName::Num(number) => number_array_index(number.value),
      PropName::BigInt(big_int) => array_index_of(&big_int.value.to_string()),
      _ => None,
    },
    _ => None,
  }
}

/// Reorders an object's own properties the way JavaScript enumerates them:
/// every array-index key first in ascending numeric order, then every other key
/// in insertion order.
///
/// Not a detail of the object literal's spelling. The order properties come out
/// in is the order their declarations reach the stylesheet, so it decides which
/// of two rules at equal specificity wins -- and `{ color: 'red', ...['a'] }`
/// was emitting `color` before `0` where the language, and so upstream, puts
/// `0` first.
///
/// Stable within each group, so the insertion order of the string keys is
/// preserved exactly.
///
/// Partitioned rather than sorted, and measurably so. A list splits in one pass
/// where a stable sort costs `n log n` plus the storage to remember each rank,
/// and over a 26-property object with two index keys the sort measured 1.30x the
/// time. An ordered map cannot be split in place, which is why `order_own_map_keys`
/// sorts instead.
///
/// The keys are read through `prop_array_index`, which asks only what this needs
/// and so builds no name. Against reading them through `prop_key` that measured
/// 0.88x on an object keyed by numbers -- the shape `{ 0: x }` parses to -- and
/// no worse on any other.
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

/// Reorders an ordered map the way JavaScript enumerates the own keys of the
/// object it stands for, given how to read the name each entry is keyed by.
///
/// The same rule `order_own_keys` applies to a property list. A `create` call
/// collects its namespaces into a map rather than a property list, and the order
/// they enumerate in decides which namespace is compiled first -- so it decides
/// the order whole rule sets reach the stylesheet.
///
/// `name_of` answers `None` for a key that spells no readable name, which then
/// ranks with the ordinary string keys.
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

/// Keeps the last property declared under each name, in the position that name
/// **first** took, and drops every property that declares no readable name.
///
/// This is what the language does. `Object.entries({ color: 1, opacity: 2,
/// color: 3 })` gives `[["color", 3], ["opacity", 2]]`: the repeated key takes
/// the later value and stays where it first appeared.
///
/// The function read the properties backwards before, and kept the first one
/// it then met. That gives the right value in the wrong place, so the two
/// declarations above came out as `opacity, color`. [`assign_props`] beside
/// this one states the same rule and already followed it.
pub fn remove_duplicates(props: Vec<PropOrSpread>) -> Vec<PropOrSpread> {
  // The place each key already took, so a repeated key is found by one lookup
  // instead of a walk of the properties placed so far.
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

/// The properties of `{ ...old_props, ...new_props }`.
///
/// This is `Object.assign`, which is what the reference implementation calls at
/// the spread and so is what "merge" has to mean here: shallow. A repeated key
/// takes the later value and keeps the position the key first took, so
/// `{ ...{ a: 1, b: 2 }, ...{ a: 3 } }` is `{ a: 3, b: 2 }` -- `a` is third in
/// no ordering.
///
/// Not a deep merge, despite what this used to be called: `{ ...{ a: { x: 1 } },
/// ...{ a: { y: 2 } } }` is `{ a: { y: 2 } }` in the language, and the nested
/// `x` is gone rather than merged in. Nesting is what a style object is mostly
/// made of -- a pseudo, an at-rule -- so a deep merge here quietly kept
/// declarations the source had replaced.
///
/// It also used to reverse. The result was assembled by pushing the old
/// properties onto the new and reversing the whole thing, which put the groups
/// in the right order and every property inside them in the wrong one, so
/// `{ ...{ color: 'red', opacity: 1 } }` emitted its two rules back to front.
/// One property is the common case and hid it.
pub fn assign_props(
  old_props: Vec<PropOrSpread>,
  new_props: Vec<PropOrSpread>,
) -> Vec<PropOrSpread> {
  let mut merged: Vec<PropOrSpread> = Vec::with_capacity(old_props.len() + new_props.len());
  // The position each key already took, so a repeated one is found by lookup
  // rather than by re-reading every property placed so far -- which was
  // quadratic, and cloned an `Atom` per comparison, on the path every object
  // spread goes through.
  let mut positions: FxHashMap<Atom, usize> = FxHashMap::default();

  for prop in old_props.into_iter().chain(new_props) {
    let Some(key) = prop_key(&prop) else {
      // A spread, a getter, a computed key: nothing here can say which key it
      // would land on, so it keeps its place and collides with nothing.
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
