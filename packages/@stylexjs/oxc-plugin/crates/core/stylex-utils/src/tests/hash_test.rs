//! Tests for the hashing helpers, with shared parsing, tower-building and byte-counting helpers.


#[cfg(test)]
mod stable_hash_tests {
  use crate::hash::stable_hash;

  #[test]
  fn returns_consistent_hash_for_same_value() {
    assert_eq!(stable_hash(&42u64), stable_hash(&42u64));
  }
}

#[cfg(test)]
mod wide_hasher_tests {
  use std::hash::Hasher;

  use super::super::WideHasher;

  #[test]
  fn finish_is_a_different_number_from_either_half_of_the_wide_one() {
    // xxh3's 64-bit digest is a separate construction from its 128-bit one.
    let mut hasher = WideHasher::new();
    hasher.write(b"stylex");

    let wide = hasher.finish_wide();
    let narrow = u128::from(hasher.finish());

    assert_ne!(narrow, wide & u128::from(u64::MAX));
    assert_ne!(narrow, wide >> 64);

    let mut again = WideHasher::new();
    again.write(b"stylex");

    assert_eq!(hasher.finish(), again.finish());
    assert_eq!(wide, again.finish_wide());
  }
}

