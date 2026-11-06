//! Workspace-wide re-exports for the collection types we use everywhere.
//!
//! Use these instead of reaching into `rustc-hash` / `indexmap` directly so
//! that swapping hashers or container types in the future is a single-file
//! change.
//!
//! ## Choice rule
//!
//! - [`FxHashMap`] / [`FxHashSet`]: default — when iteration order is **not**
//!   observed downstream. Required by `AGENTS.md`.
//! - [`FxIndexMap`] / [`FxIndexSet`]: when insertion order **is** observed
//!   (CSS rule emission, debug snapshots, etc.). Document the reason with a
//!   `// JS-parity:` comment citing the JS source line.

pub use indexmap::{IndexMap, IndexSet, map::Entry as IndexMapEntry};
pub use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet, FxHasher};

/// `IndexMap` keyed by an `Fx` hasher — matches the workspace default
/// hashing strategy while preserving insertion order.
pub type FxIndexMap<K, V> = IndexMap<K, V, FxBuildHasher>;

/// `IndexSet` with `Fx` hashing.
pub type FxIndexSet<T> = IndexSet<T, FxBuildHasher>;
