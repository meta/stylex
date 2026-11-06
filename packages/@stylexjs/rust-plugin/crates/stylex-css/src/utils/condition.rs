use crate::utils::pseudo::is_pseudo_selector;

/// Whether a key opens a nested block rather than declaring a property.
///
/// Three prefixes do that, and they are one rule between them: a colon starts a
/// pseudo selector ([`is_pseudo_selector`] — `:hover`, `::before`), an `@`
/// starts an at-rule (`@media …`), and a `[` starts an attribute selector
/// (`[data-active]`). Anything else is a property name, and its value is a
/// declaration.
///
/// Sites testing a *subset* of the three do not call this — the create-arg
/// walker and dynamic-style functions exclude attribute selectors, and pre-rule
/// nesting handles at-rules on a separate pass. Those omissions are load-bearing,
/// so widening them to this predicate would change behavior.
#[inline]
pub fn is_conditional_key(key: &str) -> bool {
  is_pseudo_selector(key) || key.starts_with('@') || key.starts_with('[')
}
