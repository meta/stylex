//! What the cache answers when a lock is lost, and one merge run from inside
//! the crate.
//!
//! Every case about what a merge *answers* lives in `tests/styleq_test.rs`,
//! which reads the crate the way a consumer does. What is here is what only
//! this side can reach: the private cache handles, and the recovery a poisoned
//! lock takes.

use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::capturing_logger::logged_at;

/// Calling the `_assert_cache_send_sync` helper covers its body in tests
/// while still serving its compile-time purpose of asserting `Send`+`Sync`
/// for `CacheEntry`/`CacheKey` (relevant when StyleX is invoked from
/// parallel processors like Rayon/Tokio).
#[test]
fn cache_types_are_send_and_sync() {
  super::_assert_cache_send_sync();
}

/// One merge, run from inside the crate.
///
/// Every case about a merge lives in `tests/styleq_test.rs`, and this does
/// not duplicate one -- it is here so the walk is *instantiated* in this
/// binary. The cases beside it build a `Styleq` and never call it, so the
/// generic methods were emitted here as an uninstantiated shell whose
/// counters are all zero, and the coverage gate read that shell as a region
/// no test runs.
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

#[test]
fn cache_lock_recovers_from_poisoned_rwlock() {
  let styleq = create_styleq::<crate::StyleValue>(StyleqOptions::default());

  let result = catch_unwind(AssertUnwindSafe(|| {
    let _guard = styleq.cache.entries.write();
    panic!("poison cache rwlock");
  }));

  assert!(result.is_err());
  assert!(styleq.cache_read(&styleq.cache).is_empty());
}

/// After the cache RwLock has been poisoned by a panicking writer, the
/// **write** path (`cache_write` + `recover_poisoned_write`) must also
/// recover and let subsequent inserts succeed. Without this test, the
/// poisoned-write recovery branch (`unwrap_or_else(...)` in `cache_write`)
/// is never exercised, leaving a coverage hole exactly in the recovery
/// code path most likely to silently break.
#[test]
fn cache_write_recovers_from_poisoned_rwlock() {
  let styleq = create_styleq::<crate::StyleValue>(StyleqOptions::default());

  let result = catch_unwind(AssertUnwindSafe(|| {
    let _guard = styleq.cache.entries.write();
    panic!("poison cache rwlock for write recovery");
  }));
  assert!(result.is_err(), "writer panic should propagate");

  // Sanity: the lock is now poisoned for both read and write.
  assert!(styleq.cache.entries.read().is_err());
  assert!(styleq.cache.entries.write().is_err());

  // `insert_cache_entry` goes through `cache_write` → must transparently
  // recover from the poisoned lock and complete the insert.
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

/// The `poison_warned` latch must flip exactly once: the first poisoned
/// recovery is logged at `error!` (actionable), every later one falls back
/// to `debug!` so a single panicked writer can't flood the log under load.
///
/// The messages are read back, because the latch is only observable in what
/// was written: the flag alone cannot tell a second `error!` from the
/// `debug!` that has to replace it.
#[test]
fn poison_warning_latches_after_first_recovery() {
  let styleq = create_styleq::<crate::StyleValue>(StyleqOptions::default());

  assert!(
    !styleq.poison_warned.load(Ordering::Relaxed),
    "freshly-built Styleq must not have its poison flag set"
  );

  let _ = catch_unwind(AssertUnwindSafe(|| {
    let _guard = styleq.cache.entries.write();
    panic!("poison cache rwlock for latch test");
  }));

  let messages = logged_at(log::Level::Debug, || {
    // First recovery: latches the flag and emits `error!`.
    drop(styleq.cache_read(&styleq.cache));
    assert!(
      styleq.poison_warned.load(Ordering::Relaxed),
      "first recovery must latch the poison-warned flag"
    );

    // Second recovery: same path, no re-latch (still true). This call is
    // what would previously have produced a second `error!` log line; now
    // it's demoted to `debug!` and the flag stays unchanged.
    drop(styleq.cache_read(&styleq.cache));
    assert!(
      styleq.poison_warned.load(Ordering::Relaxed),
      "subsequent recoveries must keep the flag set without flipping it back"
    );
  });

  assert_eq!(
    messages,
    vec![
      "styleq: cache RwLock was poisoned (read); continuing with inner cache.".to_string(),
      "styleq: cache RwLock still poisoned (read); recovered transparently.".to_string(),
    ],
    "the first recovery is the loud one and every later one is quiet"
  );
}

/// The write path names itself, so a reader of the log can tell which guard
/// was recovered. The kind is an argument of the message, which is built only
/// while a logger admits the level.
#[test]
fn a_recovered_write_guard_names_itself() {
  let styleq = create_styleq::<crate::StyleValue>(StyleqOptions::default());

  let _ = catch_unwind(AssertUnwindSafe(|| {
    let _guard = styleq.cache.entries.write();
    panic!("poison cache rwlock for the write path");
  }));

  let messages = logged_at(log::Level::Error, || {
    drop(styleq.cache_write(&styleq.cache))
  });

  assert_eq!(
    messages,
    vec!["styleq: cache RwLock was poisoned (write); continuing with inner cache.".to_string()]
  );
}

/// What the narrow digest of [`WideHasher`] answers.
///
/// `Hasher` asks every hasher for a 64-bit digest, so the type has to give
/// one, and nothing in the crate reads it: a cache key is the 128-bit digest.
/// A caller that reaches for the familiar `finish` gets that narrow answer, so
/// these cases hold it to xxh3's own 64-bit digest of the same stream, and
/// show that it is not the low half of the wide one.
mod wide_hasher {
  use super::*;
  use xxhash_rust::xxh3::{xxh3_64, xxh3_128};

  /// A hasher fed one piece per write, which is how [`hash_style`] feeds it.
  fn fed_with(pieces: &[&[u8]]) -> WideHasher {
    let mut hasher = WideHasher::default();

    for piece in pieces {
      hasher.write(piece);
    }

    hasher
  }

  #[test]
  fn the_narrow_digest_is_xxh3_s_own() {
    let hasher = fed_with(&[b"color", b"color-red"]);

    assert_eq!(
      hasher.finish(),
      xxh3_64(b"colorcolor-red"),
      "the narrow digest must be xxh3's 64-bit digest of the same stream"
    );
  }

  #[test]
  fn the_narrow_digest_is_not_the_low_half_of_the_wide_one() {
    let hasher = fed_with(&[b"color", b"color-red"]);

    assert_ne!(
      hasher.finish(),
      hasher.finish_wide() as u64,
      "the two digests are separate constructions, not one truncated to the other"
    );
  }

  #[test]
  fn the_wide_digest_is_xxh3_s_own() {
    let hasher = fed_with(&[b"color", b"color-red"]);

    assert_eq!(hasher.finish_wide(), xxh3_128(b"colorcolor-red"));
  }

  #[test]
  fn an_empty_stream_still_has_both_digests() {
    let hasher = WideHasher::default();

    assert_eq!(hasher.finish(), xxh3_64(b""));
    assert_eq!(hasher.finish_wide(), xxh3_128(b""));
  }

  #[test]
  fn a_write_of_no_bytes_leaves_both_digests_alone() {
    let empty = WideHasher::default();
    let written = fed_with(&[b"", b"", b""]);

    assert_eq!(written.finish(), empty.finish());
    assert_eq!(written.finish_wide(), empty.finish_wide());
  }

  #[test]
  fn many_small_writes_answer_what_one_large_write_answers() {
    // 512 pieces of one byte each, against the same bytes handed over once.
    let joined: Vec<u8> = (0..512u16).map(|byte| byte as u8).collect();
    let pieces: Vec<&[u8]> = joined.chunks(1).collect();

    let piecewise = fed_with(&pieces);
    let whole = fed_with(&[&joined]);

    assert_eq!(piecewise.finish(), whole.finish());
    assert_eq!(piecewise.finish_wide(), whole.finish_wide());
    assert_eq!(whole.finish(), xxh3_64(&joined));
  }

  #[test]
  fn a_stream_larger_than_any_style_is_read_whole() {
    // Four mebibytes, far past anything a style map holds, to show that the
    // hasher answers on a stream no cache key will ever carry.
    let bulk = vec![b'x'; 4 * 1024 * 1024];
    let hasher = fed_with(&[&bulk]);

    assert_eq!(hasher.finish(), xxh3_64(&bulk));
    assert_eq!(hasher.finish_wide(), xxh3_128(&bulk));
  }

  #[test]
  fn reading_a_digest_does_not_consume_the_stream() {
    let mut hasher = fed_with(&[b"width"]);

    let first = hasher.finish();
    assert_eq!(hasher.finish(), first, "the narrow digest is repeatable");
    assert_eq!(
      hasher.finish_wide(),
      xxh3_128(b"width"),
      "reading the narrow digest must not disturb the wide one"
    );

    // The stream carries on from where it was, rather than restarting.
    hasher.write(b"10px");

    assert_eq!(hasher.finish(), xxh3_64(b"width10px"));
  }

  #[test]
  fn two_streams_that_differ_answer_differently() {
    let red = fed_with(&[b"color", b"color-red"]);
    let blue = fed_with(&[b"color", b"color-blue"]);

    assert_ne!(red.finish(), blue.finish());
    assert_ne!(red.finish_wide(), blue.finish_wide());
  }

  /// The narrow digest reads the same stream the wide one does, including the
  /// length bytes `Hash` writes for a string. This is the shape [`hash_style`]
  /// produces, so it is the shape a caller would see.
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
