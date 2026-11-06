use crate::utils::condition::is_conditional_key;

#[test]
fn all_three_nesting_prefixes_are_conditional_keys() {
  for key in [
    ":hover",
    "::before",
    "@media (min-width: 600px)",
    "@supports (display: grid)",
    "[data-active]",
  ] {
    assert!(is_conditional_key(key), "is_conditional_key({key:?})");
  }
}

#[test]
fn property_names_are_not_conditional_keys() {
  for key in ["", "color", "backgroundColor", "--color", "default"] {
    assert!(!is_conditional_key(key), "is_conditional_key({key:?})");
  }
}

#[test]
fn a_prefix_only_counts_when_it_leads_the_key() {
  for key in [" @media screen", "color:hover", "x[data-active]"] {
    assert!(!is_conditional_key(key), "is_conditional_key({key:?})");
  }
}
