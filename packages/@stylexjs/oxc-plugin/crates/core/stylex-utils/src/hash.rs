use std::{
  collections::hash_map::DefaultHasher,
  hash::{Hash, Hasher},
};

use xxhash_rust::xxh3::Xxh3Default;


/// Collection-length cap of the structural (span-free) AST key. The AST walk itself is
/// front-end code (it lived here over SWC's `Expr`); the cap stays shared so every front end
/// falls back at the same size.
pub const MAX_UNSPANNED_HASH_COLLECTION_LEN: usize = 128;
const BASE36_DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
const BASE62_DIGITS: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// Hashes a float value by converting to its bit representation first.
pub fn hash_f64(value: f64) -> u64 {
  let bits = value.to_bits();
  let mut hasher = DefaultHasher::new();
  bits.hash(&mut hasher);
  hasher.finish()
}

/// Runs murmur2 over the string's UTF-16 code units, each masked to its low byte, to match JS.
/// A lone surrogate from a JS string literal cannot survive into a `&str`, so it is not hashed.
#[inline]
fn murmur2_code_units(value: &str) -> u32 {
  // ASCII code units already equal their UTF-8 bytes.
  if value.is_ascii() {
    return murmur2::murmur2(value.as_bytes(), 1);
  }

  // The UTF-8 length is an upper bound on the UTF-16 length, so this reserves once.
  let mut code_units = Vec::with_capacity(value.len());
  code_units.extend(value.encode_utf16().map(|unit| (unit & 0xff) as u8));

  murmur2::murmur2(&code_units, 1)
}

/// Creates a base-36 hash of a string using murmur2.
#[inline]
pub fn create_hash(value: &str) -> String {
  to_base36(murmur2_code_units(value))
}

/// Creates a StyleX key hash without allocating through `format!`.
#[inline]
pub fn create_key_hash(namespace: &str, key: &str) -> String {
  let mut value = String::with_capacity(namespace.len() + 1 + key.len());
  value.push_str(namespace);
  value.push('.');
  value.push_str(key);

  create_hash(&value)
}

/// Returns an authored `--name` without its dashes, or else `prefix` plus the key hash.
/// Shared by `defineConsts` and `defineVars`.
#[inline]
pub fn create_authored_or_hashed_key(prefix: &str, namespace: &str, key: &str) -> String {
  match key.strip_prefix("--") {
    Some(authored) => authored.to_string(),
    None => {
      let hash = create_key_hash(namespace, key);
      let mut name = String::with_capacity(prefix.len() + hash.len());
      name.push_str(prefix);
      name.push_str(&hash);
      name
    },
  }
}

/// Writes `value` in the radix of `digits`; zero writes no digits, leaving it to the caller.
/// Panics if `buf` is too narrow for `value`.
fn to_radix<const RADIX: usize>(mut value: u32, digits: &[u8; RADIX], buf: &mut [u8]) -> String {
  let radix = RADIX as u32;
  let mut idx = buf.len();

  debug_assert!(
    u64::from(radix)
      .checked_pow(buf.len() as u32)
      .is_none_or(|capacity| capacity > u64::from(value))
  );

  while value > 0 {
    idx -= 1;
    buf[idx] = digits[(value % radix) as usize];
    value /= radix;
  }

  // `digits` is ASCII, so this never fails and avoids `unsafe` and `expect`.
  std::str::from_utf8(&buf[idx..])
    .map(str::to_owned)
    .unwrap_or_default()
}

/// Formats as base-36 in 7 digits, which covers `u32::MAX`.
fn to_base36(value: u32) -> String {
  if value == 0 {
    return "0".to_owned();
  }

  to_radix(value, BASE36_DIGITS, &mut [0u8; 7])
}

/// Formats as base-62 in 5 digits; zero yields `""`, kept because changing it renames classes.
fn to_base62(value: u32) -> String {
  to_radix(value, BASE62_DIGITS, &mut [0u8; 5])
}

/// Deterministic hash using `DefaultHasher` (SipHash-based).
pub fn stable_hash<T: Hash>(t: &T) -> u64 {
  let mut hasher = DefaultHasher::new();
  t.hash(&mut hasher);
  hasher.finish()
}

/// The structural key's hasher: xxh3 with a 128-bit digest.
/// The width matters because some consumers act on a key hit without confirming it.
pub struct WideHasher {
  state: Xxh3Default,
}

impl WideHasher {
  pub fn new() -> Self {
    Self {
      state: Xxh3Default::new(),
    }
  }

  pub fn finish_wide(&self) -> u128 {
    self.state.digest128()
  }
}

impl Default for WideHasher {
  fn default() -> Self {
    Self::new()
  }
}

impl Hasher for WideHasher {
  fn write(&mut self, bytes: &[u8]) {
    self.state.update(bytes);
  }

  /// The 64-bit xxh3 digest, which is unrelated to the low half of `finish_wide`.
  fn finish(&self) -> u64 {
    self.state.digest()
  }
}

/// [`stable_hash`] over 128 bits, for caches that act on a hit without confirming it.
pub fn stable_hash_wide<T: Hash>(t: &T) -> u128 {
  let mut hasher = WideHasher::new();
  t.hash(&mut hasher);
  hasher.finish_wide()
}

/// Creates a short base-62 hash of a string using murmur2.
pub fn create_short_hash(value: &str) -> String {
  let hash = murmur2_code_units(value) % (62u32.pow(5));
  to_base62(hash)
}
#[cfg(test)]
#[path = "tests/hash_test.rs"]
mod tests;
