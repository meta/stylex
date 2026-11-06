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

// JS-parity: styleq/src/styleq.js — the cache is a chain: each entry has its own child map, and a
// cached chunk is only valid behind the styles that preceded it.
//
// Keys are a style array's address (valid only while the array lives) or a structural hash.
// Nothing evicts, so a long-lived `Styleq` can read a freed address as a hit and grows without
// bound. This compiler disables the cache and drops the merger per merge; a caller enabling it
// must handle both.
struct CacheEntry {
  class_name: Arc<str>,
  defined_properties: Arc<[Arc<str>]>,
  debug_string: Arc<str>,
  /// The styles cached after this one; built on first descent.
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
  /// The structural key; 128 bits because a collision would silently emit another style's classes.
  Hash(u128),
}

pub struct Styleq<V: StyleqValue> {
  options: StyleqOptions<V>,
  cache: Arc<CacheNode>,
  /// Latched after the first poisoned-lock recovery so later ones log at `debug!`.
  poison_warned: AtomicBool,
}

// Compile-time check that the cache is `Send + Sync`; test-only so coverage stays at 100%.
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
    // Membership-only set; iteration order is never observed.
    let mut defined_properties: FxHashSet<Arc<str>> = FxHashSet::default();
    let mut class_name = String::new();
    let mut inline_style: Option<StyleMap<V>> = None;
    let mut debug_string = String::new();
    // The node the next style is looked up in; `None` once an inline style stops caching.
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
    // Computed only while the walk may still cache, to avoid hashing for nothing.
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
          // Check by borrow first; already-defined properties are the common case.
          if !defined_properties.contains(prop.as_str()) {
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

        // Same initializer as the hit path.
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
      // Matches upstream: an undefined value writes nothing, defines nothing, and keeps the chain.
      if value.is_undefined() {
        continue;
      }

      if !defined_properties.contains(prop.as_str()) {
        if !value.is_null() {
          sub_style
            .get_or_insert_with(StyleMap::new)
            .insert(prop.clone(), value.clone());
        }

        defined_properties.insert(Arc::from(prop.as_str()));
        // The chain doesn't key on inline styles, so the rest of this merge is not cached.
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

  /// Logs poisoning at `error!` once per instance, then at `debug!`.
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

  /// Stores the entry (first one under a key wins) and returns the handle to descend through.
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

/// Whether the class name already carries this chunk as whole space-separated names
/// (`xabc` must not match inside `xabcdef`).
fn carries_chunk(class_name: &str, chunk: &str) -> bool {
  class_name.match_indices(chunk).any(|(start, _)| {
    let end = start + chunk.len();

    // Whole when a space or edge sits on each side; byte reads are safe since a space is one byte.
    (start == 0 || class_name.as_bytes()[start - 1] == b' ')
      && (end == class_name.len() || class_name.as_bytes()[end] == b' ')
  })
}

/// The structural key of one style (JS-parity: `styleq/src/styleq.js#L100`); 128-bit xxh3 so
/// collisions are negligible (see [`CacheKey::Hash`]).
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

  /// xxh3's 64-bit digest, required by `Hasher`; not the low half of
  /// [`WideHasher::finish_wide`], and unused here.
  fn finish(&self) -> u64 {
    self.state.digest()
  }
}

#[cfg(test)]
#[path = "tests/styleq_tests.rs"]
mod tests;
