use std::hash::Hash;

use rustc_hash::FxHashMap;

/// Buckets collection handles by a narrowing key; callers must confirm equality.
/// The key must not distinguish values that the confirming equality treats as equal.
#[derive(Clone, Debug)]
pub struct CandidateIndex<K, H> {
  buckets: FxHashMap<K, Vec<H>>,
}

impl<K, H> Default for CandidateIndex<K, H> {
  fn default() -> Self {
    Self {
      buckets: FxHashMap::default(),
    }
  }
}

impl<K: Eq + Hash, H: PartialEq> CandidateIndex<K, H> {
  /// Records a handle once per key, using a linear duplicate check within the bucket.
  pub fn record(&mut self, key: K, handle: H) {
    let bucket = self.buckets.entry(key).or_default();

    if !bucket.contains(&handle) {
      bucket.push(handle);
    }
  }

  /// Removes the handle and drops its bucket when empty.
  pub(crate) fn forget(&mut self, key: &K, handle: &H) {
    let Some(bucket) = self.buckets.get_mut(key) else {
      return;
    };

    bucket.retain(|candidate| candidate != handle);

    if bucket.is_empty() {
      self.buckets.remove(key);
    }
  }

  /// Moves a handle between optional keys, forgetting first so same-key moves retain it.
  pub fn move_entry(&mut self, replaced: Option<K>, recorded: Option<K>, handle: H) {
    if let Some(replaced) = replaced {
      self.forget(&replaced, &handle);
    }

    if let Some(recorded) = recorded {
      self.record(recorded, handle);
    }
  }

  /// Returns candidates to confirm; computes the key only when the index is nonempty.
  pub fn candidates(&self, key_of: impl FnOnce() -> K) -> &[H] {
    if self.buckets.is_empty() {
      return &[];
    }

    self.buckets.get(&key_of()).map_or(&[], Vec::as_slice)
  }
}
