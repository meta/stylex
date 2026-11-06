#[cfg(test)]
mod find_and_swap_remove_tests {
  use crate::collection::find_and_swap_remove;

  #[test]
  fn removes_matching_element() {
    let mut v = vec![1, 2, 3, 4];
    let removed = find_and_swap_remove(&mut v, |&x| x == 3);
    assert_eq!(removed, Some(3));
    assert_eq!(v.len(), 3);
    assert!(!v.contains(&3));
  }

  #[test]
  fn returns_none_when_not_found() {
    let mut v = vec![1, 2, 3];
    assert_eq!(find_and_swap_remove(&mut v, |&x| x == 99), None);
    assert_eq!(v.len(), 3);
  }

  #[test]
  fn removes_first_match_only() {
    let mut v = vec![1, 2, 2, 3];
    find_and_swap_remove(&mut v, |&x| x == 2);
    // One 2 should remain
    assert_eq!(v.iter().filter(|&&x| x == 2).count(), 1);
  }

  #[test]
  fn handles_empty_vec() {
    let mut v: Vec<i32> = vec![];
    assert_eq!(find_and_swap_remove(&mut v, |_| true), None);
  }

  #[test]
  fn handles_single_element() {
    let mut v = vec![42];
    assert_eq!(find_and_swap_remove(&mut v, |&x| x == 42), Some(42));
    assert!(v.is_empty());
  }
}

#[cfg(test)]
mod get_hash_map_difference_tests {
  use crate::collection::get_hash_map_difference;
  use rustc_hash::FxHashMap;

  #[test]
  fn returns_empty_for_identical_maps() {
    let mut a = FxHashMap::default();
    a.insert("key", 1);
    let b = a.clone();
    let diff = get_hash_map_difference(&a, &b);
    assert!(diff.is_empty());
  }

  #[test]
  fn returns_entries_only_in_first() {
    let mut a = FxHashMap::default();
    a.insert("a", 1);
    a.insert("b", 2);
    let mut b = FxHashMap::default();
    b.insert("a", 1);
    let diff = get_hash_map_difference(&a, &b);
    assert_eq!(diff.get("b"), Some(&2));
  }

  #[test]
  fn returns_entries_only_in_second() {
    let a = FxHashMap::default();
    let mut b = FxHashMap::default();
    b.insert("x", 10);
    let diff = get_hash_map_difference(&a, &b);
    assert_eq!(diff.get("x"), Some(&10));
  }

  #[test]
  fn returns_entries_with_different_values() {
    let mut a = FxHashMap::default();
    a.insert("key", 1);
    let mut b = FxHashMap::default();
    b.insert("key", 2);
    let diff = get_hash_map_difference(&a, &b);
    // Both maps have "key" with different values, so "key" appears (with orig
    // value)
    assert!(diff.contains_key("key"));
  }

  #[test]
  fn handles_empty_maps() {
    let a: FxHashMap<&str, i32> = FxHashMap::default();
    let b: FxHashMap<&str, i32> = FxHashMap::default();
    assert!(get_hash_map_difference(&a, &b).is_empty());
  }
}

#[cfg(test)]
mod get_hash_map_value_difference_tests {
  use crate::collection::get_hash_map_value_difference;
  use rustc_hash::FxHashMap;
  use swc_core::atoms::Atom;

  #[test]
  fn returns_difference_for_changed_values() {
    let mut a = FxHashMap::default();
    a.insert(Atom::from("x"), 10i16);
    let mut b = FxHashMap::default();
    b.insert(Atom::from("x"), 3i16);
    let diff = get_hash_map_value_difference(&a, &b);
    assert_eq!(diff.get(&Atom::from("x")), Some(&7));
  }

  #[test]
  fn keeps_entries_only_in_first() {
    let mut a = FxHashMap::default();
    a.insert(Atom::from("only_a"), 5i16);
    let b = FxHashMap::default();
    let diff = get_hash_map_value_difference(&a, &b);
    assert_eq!(diff.get(&Atom::from("only_a")), Some(&5));
  }

  #[test]
  fn ignores_entries_only_in_second() {
    let a = FxHashMap::default();
    let mut b = FxHashMap::default();
    b.insert(Atom::from("only_b"), 5i16);
    let diff = get_hash_map_value_difference(&a, &b);
    assert!(diff.is_empty());
  }

  #[test]
  fn skips_equal_values() {
    let mut a = FxHashMap::default();
    a.insert(Atom::from("same"), 3i16);
    let b = a.clone();
    let diff = get_hash_map_value_difference(&a, &b);
    assert!(diff.is_empty());
  }
}

#[cfg(test)]
mod sum_hash_map_values_tests {
  use crate::collection::sum_hash_map_values;
  use rustc_hash::FxHashMap;
  use swc_core::atoms::Atom;

  #[test]
  fn sums_overlapping_keys() {
    let mut a = FxHashMap::default();
    a.insert(Atom::from("x"), 2i16);
    let mut b = FxHashMap::default();
    b.insert(Atom::from("x"), 3i16);
    let sum = sum_hash_map_values(&a, &b);
    assert_eq!(sum.get(&Atom::from("x")), Some(&5));
  }

  #[test]
  fn keeps_non_overlapping_keys() {
    let mut a = FxHashMap::default();
    a.insert(Atom::from("a"), 1i16);
    let mut b = FxHashMap::default();
    b.insert(Atom::from("b"), 2i16);
    let sum = sum_hash_map_values(&a, &b);
    assert_eq!(sum.get(&Atom::from("a")), Some(&1));
    assert_eq!(sum.get(&Atom::from("b")), Some(&2));
  }

  #[test]
  fn handles_empty_maps() {
    let a: FxHashMap<Atom, i16> = FxHashMap::default();
    let b: FxHashMap<Atom, i16> = FxHashMap::default();
    assert!(sum_hash_map_values(&a, &b).is_empty());
  }
}
