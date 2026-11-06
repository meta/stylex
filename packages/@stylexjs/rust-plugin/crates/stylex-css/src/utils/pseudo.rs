/// Whether a selector segment names a pseudo *element*.
///
/// A `::` prefix marks a pseudo element (`::before`, `::thumb`); a single colon
/// marks a pseudo class (`:hover`, `:nth-child(2)`). Segments carrying neither
/// prefix — attribute selectors, at-rules, custom properties — are not pseudo
/// elements either.
///
/// Legacy single-colon spellings of pseudo elements (`:before`, `:after`,
/// `:first-line`, `:first-letter`) read as pseudo classes here; keys reaching
/// this predicate are expected in the modern double-colon form.
#[inline]
pub fn is_pseudo_element(key: &str) -> bool {
  key.starts_with("::")
}

/// Whether a key names a pseudo selector at all — class *or* element.
///
/// A single leading colon is the whole test, so `::before` satisfies it just as
/// `:hover` does. This is the rule the nesting logic wants: a key opening with
/// a colon introduces a nested selector rather than a declaration, whichever
/// kind of pseudo follows. Callers that must tell the two kinds apart reach for
/// [`is_pseudo_element`] or [`is_pseudo_class`] instead.
#[inline]
pub fn is_pseudo_selector(key: &str) -> bool {
  key.starts_with(':')
}

/// Whether a selector segment names a pseudo *class* — `:hover`, `:focus`,
/// `:nth-child(2)`.
///
/// One colon and no more: a `::` prefix is a pseudo element and fails this
/// test. Legacy single-colon elements (`:before`) pass it, as they do every
/// prefix test here; keys are expected in the modern double-colon form.
#[inline]
pub fn is_pseudo_class(key: &str) -> bool {
  is_pseudo_selector(key) && !is_pseudo_element(key)
}
