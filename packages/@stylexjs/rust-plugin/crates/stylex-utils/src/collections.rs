//! Workspace-wide collection types; use these instead of `rustc-hash` or `indexmap` directly.
//!
//! Use [`FxHashMap`] and [`FxHashSet`] unless insertion order is observed downstream, in which
//! case use [`FxIndexMap`] or [`FxIndexSet`] with a `// JS-parity:` comment citing the JS source.

pub use indexmap::{IndexMap, IndexSet, map::Entry as IndexMapEntry};
pub use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet, FxHasher};

/// Insertion-ordered `IndexMap` with `Fx` hashing.
pub type FxIndexMap<K, V> = IndexMap<K, V, FxBuildHasher>;

/// `IndexSet` with `Fx` hashing.
pub type FxIndexSet<T> = IndexSet<T, FxBuildHasher>;
