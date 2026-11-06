use std::hash::Hash;

use rustc_hash::FxHashMap;

/// Where the entries holding a given thing live, bucketed by a key that narrows
/// to them.
///
/// Six of the state manager's questions have the same shape: "which recorded
/// entry holds *this*?", asked once per `stylex.*` call or per identifier
/// reference the transform meets. Answered by walking the whole collection and
/// comparing whole subtrees with `eq_ignore_span`, each of those runs once per
/// question against every entry -- quadratic in the size of the module, and on a
/// 1,500-component JSX module the largest single cost in the transform.
///
/// `K` is what narrows: a structural hash where the question is about an
/// expression, a source position where it is about where something was written,
/// a name where it is about what a declarator binds. `H` is how an entry is
/// addressed in the collection beside it -- a position for a `Vec`, a name for a
/// map. The index holds no entry itself, so it cannot disagree with one about
/// its contents, only about where it is, which is what the confirmation below
/// settles.
///
/// **The key narrows and equality still decides.** Callers confirm the
/// candidates this hands back, which is the shape `adr/0005` calls "narrow a
/// bucket by hash and then confirm". Confirmation removes false positives -- a
/// hit alone would make these consumers ones for which the key *is* the
/// equality test. It cannot remove false negatives: a bucket miss is final, so
/// the key must never be *stricter* than the equality that confirms it. It is
/// stricter in three places today, all unreachable: `Ident::optional` and the
/// `raw` of `Str`/`Number`/`BigInt` are hashed but not compared, and under
/// `EQ_IGNORE_SPAN_IGNORE_CTXT` -- which nothing sets -- an identifier's
/// `SyntaxContext` would join them. Anything that sets that flag has to change
/// the key.
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
  /// Records that the entry at `handle` holds what `key` narrows to.
  ///
  /// Recording one that is already there is a no-op rather than a second bucket
  /// entry, so a collection filled twice -- which the discovery cycle does --
  /// indexes each entry once.
  ///
  /// **The scan that keeps it a no-op is linear in the bucket, so filling one
  /// key is quadratic in the entries that share it.** Timed once on an
  /// optimized build, directionally: a record costs about 14 ns at a bucket of
  /// 100, 62 ns at 1 000, 464 ns at 10 000 and 4.7 us at 100 000, where a
  /// record under a key of its own stays flat near 20 ns. A bucket only grows
  /// where two entries are *structurally identical*, so reaching the thousands
  /// takes a module holding thousands of byte-identical `stylex.create` calls,
  /// which all compile to the same class names. Left alone on that: one entry
  /// per bucket is the shape every module in this repository has, and a hash
  /// set per bucket would cost that shape a second allocation and a hash of
  /// every handle to buy nothing.
  pub fn record(&mut self, key: K, handle: H) {
    let bucket = self.buckets.entry(key).or_default();

    if !bucket.contains(&handle) {
      bucket.push(handle);
    }
  }

  /// Drops the record that the entry at `handle` holds what `key` narrows to,
  /// forgetting the bucket once nothing is left in it.
  ///
  /// Removing the emptied bucket is for [`Self::candidates`]' `is_empty()`
  /// short-circuit alone, never for correctness: an empty bucket and an absent
  /// key both answer with nothing.
  pub(crate) fn forget(&mut self, key: &K, handle: &H) {
    let Some(bucket) = self.buckets.get_mut(key) else {
      return;
    };

    bucket.retain(|candidate| candidate != handle);

    if bucket.is_empty() {
      self.buckets.remove(key);
    }
  }

  /// Moves the entry at `handle` from the key it was recorded under to the one
  /// it now belongs to, either of which may be absent.
  ///
  /// The one way an entry's key changes, and the reason it is a method rather
  /// than a pair of calls at four sites: the order matters. Forgetting first
  /// means an entry replaced by something under the *same* key keeps its record
  /// instead of losing it to the forget that would otherwise follow.
  pub fn move_entry(&mut self, replaced: Option<K>, recorded: Option<K>, handle: H) {
    if let Some(replaced) = replaced {
      self.forget(&replaced, &handle);
    }

    if let Some(recorded) = recorded {
      self.record(recorded, handle);
    }
  }

  /// The entries that may hold what `key_of` describes -- the candidates the
  /// caller confirms.
  ///
  /// The key is computed only when there is something to look it up in, because
  /// computing a structural one walks the whole expression. A module that
  /// records nothing of the kind this index holds -- every style in one
  /// top-level array is the common shape -- then answers for free, where hashing
  /// a style object first would cost more than the scan this replaces did.
  pub fn candidates(&self, key_of: impl FnOnce() -> K) -> &[H] {
    if self.buckets.is_empty() {
      return &[];
    }

    self.buckets.get(&key_of()).map_or(&[], Vec::as_slice)
  }
}
