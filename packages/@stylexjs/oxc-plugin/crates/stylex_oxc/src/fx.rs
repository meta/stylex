//! Fast hashing for every map and set in the front end (the core's `stylex_utils::collections`
//! types under short names). Keys are short strings and small integers from one file, so a
//! DoS-resistant hasher (std's SipHash) buys nothing here. Hash-map iteration order never reaches
//! the output (it was random per process under SipHash).

pub use stylex_utils::collections::FxBuildHasher;

pub type HashMap<K, V> = stylex_utils::collections::FxHashMap<K, V>;
pub type HashSet<K> = stylex_utils::collections::FxHashSet<K>;
pub type IndexMap<K, V> = stylex_utils::collections::FxIndexMap<K, V>;
pub type IndexSet<K> = stylex_utils::collections::FxIndexSet<K>;
