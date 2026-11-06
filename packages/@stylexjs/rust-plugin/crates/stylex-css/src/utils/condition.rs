use crate::utils::pseudo::is_pseudo_selector;

/// Whether a key opens a nested block (pseudo selector, `@` at-rule, or `[` attribute selector).
///
/// Some sites deliberately test only a subset of these; don't widen them to this predicate.
#[inline]
pub fn is_conditional_key(key: &str) -> bool {
  is_pseudo_selector(key) || key.starts_with('@') || key.starts_with('[')
}
