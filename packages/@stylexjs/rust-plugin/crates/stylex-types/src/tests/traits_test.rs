use stylex_structures::stylex_state_options::StyleXStateOptions;

use crate::traits::WhenMarkerValue;

/// Options answer "not a marker" to every marker test and contribute only `classNamePrefix`.
#[test]
fn options_are_not_a_marker_and_expose_only_their_prefix() {
  let options = StyleXStateOptions::default();

  assert_eq!(options.as_str_value(), None);
  assert!(!options.is_proxy());
  assert_eq!(options.as_proxy_string(), None);
  assert_eq!(options.first_css_key(), None);
  assert_eq!(options.class_name_prefix(), Some("x"));
}

/// `is_proxy` agrees with `as_proxy_string` for every prefix.
#[test]
fn is_proxy_agrees_with_as_proxy_string() {
  for prefix in ["x", "", "custom"] {
    let options = StyleXStateOptions::default().with_class_name_prefix(prefix);

    assert_eq!(options.is_proxy(), options.as_proxy_string().is_some());
  }
}

/// An empty prefix is `Some("")` (JS `classNamePrefix != null`), yielding `-default-marker`.
#[test]
fn an_empty_prefix_is_reported_as_present() {
  let options = StyleXStateOptions::default().with_class_name_prefix("");

  assert_eq!(options.class_name_prefix(), Some(""));
}
