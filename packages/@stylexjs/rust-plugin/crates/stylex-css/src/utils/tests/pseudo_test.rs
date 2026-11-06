use crate::utils::pseudo::{is_pseudo_class, is_pseudo_element, is_pseudo_selector};

/// The single classification the three predicates spell out between them: a
/// key is no pseudo at all, a pseudo class, or a pseudo element.
#[derive(Clone, Copy, Debug)]
enum Kind {
  NotPseudo,
  Class,
  Element,
}

/// Asserts all three predicates agree on a key's kind, so each test pins a
/// classification rather than one predicate's half of it.
fn assert_kind(key: &str, expected: Kind) {
  let (selector, element, class) = match expected {
    Kind::NotPseudo => (false, false, false),
    Kind::Class => (true, false, true),
    Kind::Element => (true, true, false),
  };

  assert_eq!(
    is_pseudo_selector(key),
    selector,
    "is_pseudo_selector({key:?})"
  );
  assert_eq!(
    is_pseudo_element(key),
    element,
    "is_pseudo_element({key:?})"
  );
  assert_eq!(is_pseudo_class(key), class, "is_pseudo_class({key:?})");
}

// ── pseudo elements ──────────────────────────────────────────────────

#[test]
fn a_double_colon_prefix_is_a_pseudo_element() {
  for key in ["::before", "::after", "::thumb"] {
    assert_kind(key, Kind::Element);
  }
}

/// The prefix is the whole test, so a segment that is nothing but the prefix
/// still lands on the element side rather than falling between the two.
#[test]
fn a_bare_double_colon_reads_as_a_pseudo_element() {
  assert_kind("::", Kind::Element);
  assert_kind(":::", Kind::Element);
}

// ── pseudo classes ───────────────────────────────────────────────────

#[test]
fn a_single_colon_prefix_is_a_pseudo_class() {
  for key in [":hover", ":focus", ":nth-child(2)"] {
    assert_kind(key, Kind::Class);
  }
}

#[test]
fn legacy_single_colon_elements_read_as_pseudo_classes() {
  for key in [":before", ":after", ":first-line", ":first-letter"] {
    assert_kind(key, Kind::Class);
  }
}

#[test]
fn a_bare_colon_reads_as_a_pseudo_class() {
  assert_kind(":", Kind::Class);
}

// ── everything else ──────────────────────────────────────────────────

#[test]
fn keys_without_a_colon_prefix_are_no_pseudo_at_all() {
  for key in [
    "",
    "color",
    "--color",
    "var(--color)",
    "default",
    "[data-active]",
    "@media (min-width: 600px)",
  ] {
    assert_kind(key, Kind::NotPseudo);
  }
}

#[test]
fn a_colon_only_counts_when_it_leads_the_key() {
  assert_kind(":hover::before", Kind::Class);
  assert_kind(" ::before", Kind::NotPseudo);
  assert_kind(" :hover", Kind::NotPseudo);
}
