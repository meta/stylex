//! Crate-internal tests: private cache handles and poisoned-lock recovery. Merge behaviour is
//! tested in `tests/styleq_test.rs`.

use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

/// Instantiates the generic walk in this binary so coverage sees it run.
#[test]
fn a_merge_runs_from_the_crate_s_own_binary() {
  let styleq = create_styleq::<crate::StyleValue>(StyleqOptions::default());
  let mut style = StyleMap::new();
  style.insert(COMPILED_KEY.to_string(), crate::StyleValue::Bool(true));
  style.insert("color".to_string(), crate::StyleValue::string("color-red"));

  let mut dynamic = StyleMap::new();
  dynamic.insert("width".to_string(), crate::StyleValue::string("10px"));

  let result = styleq.styleq(&[
    StyleqInput::Style(style),
    StyleqInput::Style(dynamic.clone()),
  ]);

  assert_eq!(result.class_name, "color-red");
  assert_eq!(result.inline_style, Some(dynamic));
}

/// The write path recovers from a poisoned lock and later inserts succeed.
#[test]
fn cache_write_recovers_from_poisoned_rwlock() {
  let styleq = create_styleq::<crate::StyleValue>(StyleqOptions::default());

  let result = catch_unwind(AssertUnwindSafe(|| {
    let _guard = styleq.cache.entries.write();
    panic!("poison cache rwlock for write recovery");
  }));
  assert!(result.is_err(), "writer panic should propagate");

  assert!(styleq.cache.entries.read().is_err());
  assert!(styleq.cache.entries.write().is_err());

  // Must recover from the poisoned lock and complete the insert.
  let key = CacheKey::Hash(0xDEAD_BEEF);
  let entry = CacheEntry {
    class_name: Arc::from(""),
    defined_properties: Arc::from(Vec::<Arc<str>>::new()),
    debug_string: Arc::from(""),
    next: OnceLock::new(),
  };
  styleq.insert_cache_entry(&styleq.cache, key, entry);

  let cache = styleq.cache_read(&styleq.cache);
  assert!(
    cache.contains_key(&key),
    "recovered cache must accept new entries after poisoning"
  );
}

/// [`WideHasher`]'s 64-bit `finish` is xxh3's own 64-bit digest, not the low half of the wide one.
mod wide_hasher {
  use super::*;

  /// Includes the length bytes `Hash` writes for a string, as [`hash_style`] produces.
  #[test]
  fn a_style_hashed_through_the_trait_feeds_both_digests() {
    let mut style = StyleMap::new();
    style.insert("color".to_string(), crate::StyleValue::string("color-red"));

    let mut hasher = WideHasher::default();
    for (prop, value) in &style {
      prop.hash(&mut hasher);
      value.hash(&mut hasher);
    }

    assert_eq!(
      hasher.finish_wide(),
      hash_style(&style),
      "the wide digest is the key hash_style answers"
    );
    assert_ne!(
      hasher.finish() as u128,
      hasher.finish_wide(),
      "the narrow digest is no part of that key"
    );
  }
}
