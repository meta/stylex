/// Whether a selector segment names a pseudo element (`::` prefix).
///
/// Legacy single-colon elements (`:before`) read as pseudo classes; keys must use `::`.
#[inline]
pub fn is_pseudo_element(key: &str) -> bool {
  key.starts_with("::")
}

/// Whether a key names a pseudo selector at all (class or element): a leading colon.
#[inline]
pub fn is_pseudo_selector(key: &str) -> bool {
  key.starts_with(':')
}

/// Whether a selector segment names a pseudo class (single colon, not `::`).
#[inline]
pub fn is_pseudo_class(key: &str) -> bool {
  is_pseudo_selector(key) && !is_pseudo_element(key)
}
