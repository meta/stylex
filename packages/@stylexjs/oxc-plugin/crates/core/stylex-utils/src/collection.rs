use std::hash::Hash;

use rustc_hash::FxHashMap;

/// Swap-removes and returns the first element matching `predicate`; order is not preserved.
pub fn find_and_swap_remove<T, F>(vec: &mut Vec<T>, predicate: F) -> Option<T>
where
  F: Fn(&T) -> bool,
{
  vec
    .iter()
    .position(predicate)
    .map(|index| vec.swap_remove(index))
}

/// Returns the entries that exist in only one map or whose values differ.
pub fn get_hash_map_difference<K, V>(
  orig_map: &FxHashMap<K, V>,
  compare_map: &FxHashMap<K, V>,
) -> FxHashMap<K, V>
where
  K: Eq + Hash + Clone,
  V: PartialEq + Clone,
{
  let mut diff = FxHashMap::default();

  for (key, value) in orig_map {
    if let Some(map2_value) = compare_map.get(key) {
      if value != map2_value {
        diff.insert(key.clone(), value.clone());
      }
    } else {
      diff.insert(key.clone(), value.clone());
    }
  }

  for (key, value) in compare_map {
    if !orig_map.contains_key(key) {
      diff.insert(key.clone(), value.clone());
    }
  }

  diff
}

/// Returns `orig_map` entries that differ from `map2`, valued by their difference.
/// Entries that exist only in `orig_map` keep their original value.
/// Generic over the key: SWC callers pass `Atom`, others `String` or their own atom type.
pub fn get_hash_map_value_difference<K: Eq + Hash + Clone>(
  orig_map: &FxHashMap<K, i16>,
  map2: &FxHashMap<K, i16>,
) -> FxHashMap<K, i16> {
  let mut diff = FxHashMap::default();

  for (key, value) in orig_map {
    if let Some(map2_value) = map2.get(key) {
      if value != map2_value {
        diff.insert(key.clone(), value - map2_value);
      }
    } else {
      diff.insert(key.clone(), *value);
    }
  }

  diff
}

/// Merges two hash maps by summing their values.
pub fn sum_hash_map_values<K: Eq + Hash + Clone>(
  orig_map: &FxHashMap<K, i16>,
  compare_map: &FxHashMap<K, i16>,
) -> FxHashMap<K, i16> {
  let mut sum_map = FxHashMap::default();

  for (key, value) in orig_map {
    sum_map.insert(key.clone(), *value);
  }

  for (key, value) in compare_map {
    sum_map
      .entry(key.clone())
      .and_modify(|e| *e += value)
      .or_insert(*value);
  }

  sum_map
}

#[cfg(test)]
#[path = "tests/collection_test.rs"]
mod tests;
