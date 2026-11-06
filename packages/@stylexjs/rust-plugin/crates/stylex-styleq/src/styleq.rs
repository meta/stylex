use std::{
  hash::{Hash, Hasher},
  sync::{
    Arc, OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard,
    atomic::{AtomicBool, Ordering},
  },
};

use log::{debug, error};
use rustc_hash::{FxHashMap, FxHashSet};
use stylex_constants::constants::common::COMPILED_KEY;
use xxhash_rust::xxh3::Xxh3Default;

use crate::{StyleMap, StyleqArgument, StyleqInput, StyleqOptions, StyleqResult, StyleqValue};

// JS-parity: styleq/src/styleq.js — the cache is a chain, not one map. Every
// entry carries its own child map, and the walk descends into it, so a style
// cached after another style is found under that other style and never at the
// root.
//
// The chain is what makes a cached chunk true. A chunk holds only the
// properties the styles after it had not already defined, so it is cut for one
// merge and says nothing about any other. One flat map conflates them: a style
// that contributed nothing behind another was read back as contributing
// nothing on its own, and a style cached on its own was read back behind
// another and defined its properties twice.
//
// Keys are either the source array reference or a structural hash, and each
// carries a caveat a caller has to answer:
//
// - A reference key is the address of the style array, which names that array
//   only while the array is alive. Nothing here evicts, so a `Styleq` that
//   outlives the styles it cached can read a freed address back as a hit for
//   whatever was put there next.
// - Nothing evicts, and the chain holds one entry per distinct walked suffix
//   where the flat map held one per style. The reference keeps `WeakMap`s, so
//   its entries die with the style objects; these do not, so a `Styleq` kept
//   across many merges keeps every path it walked.
//
// Both are about how long one `Styleq` lives, and this compiler answers both by
// not caching at all. The merger it builds has the cache off and is dropped when
// the merge returns, so it stores no key that can outlive its style and no chain
// that can collect entries.
//
// A merger that lives longer does answer lookups from the cache. What it saves
// is less than one percent, and only with one of the two keys it could use.
// Over the transform suite and the fixture corpus, a merger that lives
// for the file answers 24% of its lookups from the cache, and one that lives
// for the whole process answers 64%. The merger built per merge answers none.
// Both keys were then timed against a module built to repeat: 100 components,
// each reading its styles from nine `stylex.props` sites, where a file-scoped
// cache answers 89% of its lookups.
//
// - The hash key made that module 2.5% slower than no cache at all, because the
//   key walks every property of the style, which is the work a hit saves.
// - The address key made it 0.7% faster, and made a module that repeats nothing
//   0.9% slower.
//
// The two legs of each pair ran in one process, so they compare with each
// other. They are not the release gate, which is a paired `bench:revisions` run
// on a runner. That module hits nine times in ten where the corpus hits about
// one in four, so a file of this corpus lands nearer the leg that repeats
// nothing, which lost. A best case worth less than one percent does not pay for
// the two caveats above.
//
// The address caveat also asks more of a caller than it looks. The compiler
// knows that a style it read from the state lives for the file, and that a
// style it built for one argument is dropped with the merge. A caller that kept
// a merger would have to give an address key for the first kind and refuse it
// for the second.
//
// A caller that wants the cache must answer both caveats.
//
// Order is never observed downstream, so an unordered FxHashMap is appropriate.
// The entry itself is wrapped in `Arc` so cache hits are a refcount bump rather
// than a deep clone of three owned strings + a `Vec<Arc<str>>`.
struct CacheEntry {
  class_name: Arc<str>,
  defined_properties: Arc<[Arc<str>]>,
  debug_string: Arc<str>,
  /// The styles cached after this one, which is the node the walk descends
  /// into.
  ///
  /// The map is built on the first descent, by the same initialiser whether
  /// the walk reached the entry as a hit or as a miss. One rule rather than
  /// two: filling it eagerly on the miss path made the hit path's initialiser
  /// dead and forced an arm for a `set` that cannot fail.
  ///
  /// Every walk that stores an entry then descends through it, so this costs
  /// the same map the eager form did. What it removes is the second way of
  /// getting one.
  next: OnceLock<Arc<CacheNode>>,
}

/// One level of the cache chain.
#[derive(Default)]
struct CacheNode {
  entries: RwLock<FxHashMap<CacheKey, Arc<CacheEntry>>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum CacheKey {
  Identity(usize),
  /// The structural key, 128 bits wide.
  ///
  /// Nothing compares the style to the entry a hit came from, so the key is the
  /// only thing that says the entry is this style's. A 64-bit key made a
  /// collision answer with another style's class names and defined properties
  /// -- wrong CSS, and no sign of it. Width is what removes that, and it is
  /// what `stylex-utils` already gives every other index whose reads act on a
  /// hit without confirming it.
  Hash(u128),
}

pub struct Styleq<V: StyleqValue> {
  options: StyleqOptions<V>,
  cache: Arc<CacheNode>,
  /// Latched after the first poisoned-lock recovery so subsequent recoveries
  /// log at `debug!` level instead of flooding `error!` once per call. The
  /// first occurrence is still surfaced as an error (the actionable signal).
  poison_warned: AtomicBool,
}

// Compile-time guarantee that the cache layer is safe to share across threads
// (relevant for parallel SWC processing of multiple files via Rayon/Tokio).
//
// Implemented as a `#[cfg(test)]`-only function so the `Send`/`Sync` trait
// bounds are still type-checked during normal compilation **and** the body is
// only emitted in test builds where it can be reached, keeping coverage at
// 100% without needing `#[coverage(off)]` (which is unstable on consts).
#[cfg(test)]
#[allow(dead_code)]
fn _assert_cache_send_sync() {
  fn assert_send<T: Send>() {}
  fn assert_sync<T: Sync>() {}
  assert_send::<CacheEntry>();
  assert_sync::<CacheEntry>();
  assert_send::<CacheNode>();
  assert_sync::<CacheNode>();
  assert_send::<CacheKey>();
  assert_sync::<CacheKey>();
}

pub fn create_styleq<V: StyleqValue>(options: StyleqOptions<V>) -> Styleq<V> {
  Styleq {
    options,
    cache: Arc::default(),
    poison_warned: AtomicBool::new(false),
  }
}

pub fn styleq<V: StyleqValue>(styles: &[StyleqInput<V>]) -> StyleqResult<V> {
  create_styleq(StyleqOptions::default()).styleq(styles)
}

impl<V: StyleqValue> Styleq<V> {
  pub fn styleq<A>(&self, arguments: &[A]) -> StyleqResult<V>
  where
    A: StyleqArgument<V>,
  {
    // Membership-only set (`Arc<str>` keys are cheap to clone-on-insert and
    // make the property-already-defined check O(1) instead of the previous
    // O(n) `Vec::contains`. Property iteration order is never observed
    // downstream — only "have we seen this prop?" matters here.
    let mut defined_properties: FxHashSet<Arc<str>> = FxHashSet::default();
    let mut class_name = String::new();
    let mut inline_style: Option<StyleMap<V>> = None;
    let mut debug_string = String::new();
    // The node the next compiled style is looked up in. `None` once the walk
    // may no longer cache, which an inline style that defines a new property
    // causes.
    let mut next_cache = if self.options.disable_cache {
      None
    } else {
      Some(Arc::clone(&self.cache))
    };
    let mut styles = arguments.iter().collect::<Vec<_>>();

    while let Some(possible_style) = styles.pop() {
      if possible_style.should_skip() {
        continue;
      }

      if let Some(nested_styles) = possible_style.as_nested() {
        styles.extend(nested_styles);
        continue;
      }

      let Some(style) = possible_style.as_style() else {
        continue;
      };
      let cache_key = possible_style.cache_key();

      let transformed_style;
      let style = match &self.options.transform {
        Some(transform) => {
          transformed_style = transform(style.clone());
          &transformed_style
        },
        None => style,
      };

      if style.contains_key(COMPILED_KEY) {
        self.process_compiled_style(
          style,
          &mut defined_properties,
          &mut class_name,
          &mut debug_string,
          cache_key,
          &mut next_cache,
        );
      } else if self.options.disable_mix {
        let mut next_inline_style = style.clone();

        if let Some(existing_inline_style) = inline_style.take() {
          for (prop, value) in existing_inline_style {
            next_inline_style.insert(prop, value);
          }
        }

        inline_style = Some(next_inline_style);
      } else {
        self.process_inline_style(
          style,
          &mut defined_properties,
          &mut inline_style,
          &mut next_cache,
        );
      }
    }

    StyleqResult {
      class_name,
      inline_style,
      data_style_src: debug_string,
    }
  }

  fn process_compiled_style(
    &self,
    style: &StyleMap<V>,
    defined_properties: &mut FxHashSet<Arc<str>>,
    class_name: &mut String,
    debug_string: &mut String,
    cache_key: Option<usize>,
    next_cache: &mut Option<Arc<CacheNode>>,
  ) {
    let mut class_name_chunk = String::new();
    // Named only where the walk may still cache. A merger with the cache off,
    // and a merge an inline style has closed, both read no entry and store
    // none, so a style with no address key would otherwise pay a walk of every
    // property for a name nothing asks for.
    let cache_key = next_cache.as_ref().map(|_| match cache_key {
      Some(cache_key) if self.options.transform.is_none() => CacheKey::Identity(cache_key),
      _ => CacheKey::Hash(hash_style(style)),
    });

    let cached = next_cache
      .as_ref()
      .zip(cache_key.as_ref())
      .and_then(|(node, cache_key)| self.get_cache_entry(node, cache_key));

    if let Some(cache_entry) = cached {
      class_name_chunk.push_str(&cache_entry.class_name);
      debug_string.clear();
      debug_string.push_str(&cache_entry.debug_string);
      // `Arc<str>` clone is a refcount bump — no per-element heap allocation
      // on the cache-hit fast path (was a `String::clone` per property).
      defined_properties.extend(cache_entry.defined_properties.iter().cloned());
      // Descend, so the next style is looked up behind this one.
      *next_cache = Some(Arc::clone(cache_entry.next.get_or_init(Arc::default)));
    } else {
      let use_cache = next_cache.is_some();
      let mut defined_properties_chunk: Vec<Arc<str>> = Vec::new();

      for (prop, value) in style {
        if prop == COMPILED_KEY {
          if !value.is_true_bool() {
            if let Some(compiled_key_value) = value.as_class_name() {
              if debug_string.is_empty() {
                debug_string.push_str(compiled_key_value);
              } else {
                debug_string.insert_str(0, "; ");
                debug_string.insert_str(0, compiled_key_value);
              }
            } else {
              error!(
                "styleq: {} typeof {:?} is not \"string\" or \"true\".",
                COMPILED_KEY, value
              );
            }
          }

          continue;
        }

        if value.as_class_name().is_some() || value.is_null() {
          // Asked by borrow first, as `process_inline_style` already asks it.
          // A property a style behind this one already declared is the common
          // case of a merge, and allocating its name to learn that threw the
          // allocation away again.
          if !defined_properties.contains(prop.as_str()) {
            // One `Arc<str>` for the membership set and the cache chunk both,
            // rather than a `String` and an `Arc` for the same name.
            let prop_arc: Arc<str> = Arc::from(prop.as_str());

            defined_properties.insert(Arc::clone(&prop_arc));

            if use_cache {
              defined_properties_chunk.push(prop_arc);
            }

            if let Some(value) = value.as_class_name() {
              if !class_name_chunk.is_empty() {
                class_name_chunk.push(' ');
              }

              class_name_chunk.push_str(value);
            }
          }
        } else {
          error!(
            "styleq: {} typeof {:?} is not \"string\" or \"null\".",
            prop, value
          );
        }
      }

      if let Some(node) = next_cache.take()
        && let Some(cache_key) = cache_key
      {
        let entry = self.insert_cache_entry(
          &node,
          cache_key,
          CacheEntry {
            class_name: Arc::from(class_name_chunk.as_str()),
            defined_properties: Arc::from(defined_properties_chunk.into_boxed_slice()),
            debug_string: Arc::from(debug_string.as_str()),
            next: OnceLock::new(),
          },
        );

        // Descend through the same initialiser the hit path uses, so an entry
        // has one rule for how its child map comes to exist however the walk
        // arrived at it.
        *next_cache = Some(Arc::clone(entry.next.get_or_init(Arc::default)));
      }
    }

    if !class_name_chunk.is_empty() {
      if class_name.is_empty() {
        class_name.push_str(&class_name_chunk);
      } else if !self.options.dedupe_class_name_chunks
        || !carries_chunk(class_name, &class_name_chunk)
      {
        class_name.insert(0, ' ');
        class_name.insert_str(0, &class_name_chunk);
      }
    }
  }

  fn process_inline_style(
    &self,
    style: &StyleMap<V>,
    defined_properties: &mut FxHashSet<Arc<str>>,
    inline_style: &mut Option<StyleMap<V>>,
    next_cache: &mut Option<Arc<CacheNode>>,
  ) {
    let mut sub_style: Option<StyleMap<V>> = None;

    for (prop, value) in style {
      // An undefined value is not a declaration. The reference skips the whole
      // body for one, so it writes nothing, defines nothing and does not break
      // the chain -- and a later style may still declare the property.
      if value.is_undefined() {
        continue;
      }

      // O(1) borrow-based lookup; only allocate an `Arc<str>` if the
      // property is genuinely new to the set.
      if !defined_properties.contains(prop.as_str()) {
        if !value.is_null() {
          sub_style
            .get_or_insert_with(StyleMap::new)
            .insert(prop.clone(), value.clone());
        }

        defined_properties.insert(Arc::from(prop.as_str()));
        // What follows an inline style depends on that style, which the chain
        // does not key on, so the rest of this merge is not cached.
        *next_cache = None;
      }
    }

    if let Some(mut sub_style) = sub_style {
      if let Some(existing_inline_style) = inline_style.take() {
        for (prop, value) in existing_inline_style {
          sub_style.entry(prop).or_insert(value);
        }
      }

      *inline_style = Some(sub_style);
    }
  }

  fn cache_read<'a>(
    &self,
    node: &'a CacheNode,
  ) -> RwLockReadGuard<'a, FxHashMap<CacheKey, Arc<CacheEntry>>> {
    node
      .entries
      .read()
      .unwrap_or_else(|poisoned| self.recover_poisoned_read(poisoned))
  }

  fn cache_write<'a>(
    &self,
    node: &'a CacheNode,
  ) -> RwLockWriteGuard<'a, FxHashMap<CacheKey, Arc<CacheEntry>>> {
    node
      .entries
      .write()
      .unwrap_or_else(|poisoned| self.recover_poisoned_write(poisoned))
  }

  /// Logs the poisoning event at `error!` exactly once per `Styleq`
  /// instance; subsequent recoveries are demoted to `debug!` so a single
  /// panicked writer can't flood the log under sustained load.
  fn report_poisoned(&self, kind: &str) {
    if !self.poison_warned.swap(true, Ordering::Relaxed) {
      error!("styleq: cache RwLock was poisoned ({kind}); continuing with inner cache.");
    } else {
      debug!("styleq: cache RwLock still poisoned ({kind}); recovered transparently.");
    }
  }

  fn recover_poisoned_read<'a>(
    &self,
    poisoned: std::sync::PoisonError<RwLockReadGuard<'a, FxHashMap<CacheKey, Arc<CacheEntry>>>>,
  ) -> RwLockReadGuard<'a, FxHashMap<CacheKey, Arc<CacheEntry>>> {
    self.report_poisoned("read");
    poisoned.into_inner()
  }

  fn recover_poisoned_write<'a>(
    &self,
    poisoned: std::sync::PoisonError<RwLockWriteGuard<'a, FxHashMap<CacheKey, Arc<CacheEntry>>>>,
  ) -> RwLockWriteGuard<'a, FxHashMap<CacheKey, Arc<CacheEntry>>> {
    self.report_poisoned("write");
    poisoned.into_inner()
  }

  fn get_cache_entry(&self, node: &CacheNode, cache_key: &CacheKey) -> Option<Arc<CacheEntry>> {
    self.cache_read(node).get(cache_key).map(Arc::clone)
  }

  /// Stores the entry and hands back the shared handle the walk descends
  /// through.
  ///
  /// The first entry stored under a key stays, and a rival a concurrent miss
  /// built is dropped. Both say the same thing -- the chunk of one style at one
  /// position of the chain -- so which of the two is kept does not matter, but
  /// keeping one of them does: overwriting left two divergent subtrees, and
  /// everything the loser's walk cached below it was orphaned where no later
  /// walk could reach it.
  fn insert_cache_entry(
    &self,
    node: &CacheNode,
    cache_key: CacheKey,
    cache_entry: CacheEntry,
  ) -> Arc<CacheEntry> {
    Arc::clone(
      self
        .cache_write(node)
        .entry(cache_key)
        .or_insert_with(|| Arc::new(cache_entry)),
    )
  }
}

/// Whether the class name already carries this chunk, whole.
///
/// Matched on the spaces around it rather than anywhere in the text. A StyleX
/// class name is a hash of no fixed length, so a short name can sit inside a
/// longer one -- `xabc` inside `xabcdef` -- and a plain text search dropped the
/// short one as a repeat. The rule it names then reached no element, with
/// nothing to show for it.
fn carries_chunk(class_name: &str, chunk: &str) -> bool {
  class_name.match_indices(chunk).any(|(start, _)| {
    let end = start + chunk.len();

    // A match is whole when a space or an edge sits on each side of it. Both
    // are read as bytes, which is safe because a space is one byte and a match
    // starts and ends on a character.
    (start == 0 || class_name.as_bytes()[start - 1] == b' ')
      && (end == class_name.len() || class_name.as_bytes()[end] == b' ')
  })
}

/// The structural key of one style.
///
/// JS-parity: `styleq/src/styleq.js#L100`, the structural hash branch. The
/// reference keys on the text of the style, which no two different styles
/// share; this keys on a hash, which two different styles can, so the hash is
/// 128 bits wide. See [`CacheKey::Hash`] for what a collision would cost.
///
/// xxh3 rather than the narrow hasher the map itself uses, because the narrow
/// one has no digest wider than 64 bits. The width is not free: measured over
/// styles of 1 to 40 properties, fed one piece at a time, xxh3 costs about
/// seven times what the narrow hasher costs. It is paid only by a caller that
/// both keeps the cache on and has no address key for its styles -- this
/// compiler has neither -- and a wrong answer costs more than a hash.
fn hash_style<V: StyleqValue>(style: &StyleMap<V>) -> u128 {
  let mut hasher = WideHasher::default();

  for (prop, value) in style {
    prop.hash(&mut hasher);
    value.hash(&mut hasher);
  }

  hasher.finish_wide()
}

/// An xxh3 hasher that answers 128 bits.
#[derive(Default)]
struct WideHasher {
  state: Xxh3Default,
}

impl WideHasher {
  fn finish_wide(&self) -> u128 {
    self.state.digest128()
  }
}

impl Hasher for WideHasher {
  fn write(&mut self, bytes: &[u8]) {
    self.state.update(bytes);
  }

  /// xxh3's 64-bit digest, present only because `Hasher` asks for it.
  ///
  /// It is **not** the low half of [`WideHasher::finish_wide`]: the two digests
  /// are separate constructions over the same stream. Nothing here reads this,
  /// and a caller that reached for the familiar `finish` would get a narrower
  /// key without being told -- which is the hazard the width exists to remove.
  fn finish(&self) -> u64 {
    self.state.digest()
  }
}

#[cfg(test)]
#[path = "tests/styleq_tests.rs"]
mod tests;
