use stylex_structures::stylex_state_options::StyleXStateOptions;

use crate::traits::WhenMarkerValue;

/// The options share the second `when` slot with a custom marker, so they have
/// to answer every marker type test with "not a marker" and contribute only
/// the `classNamePrefix` the default marker is built from.
#[test]
fn options_are_not_a_marker_and_expose_only_their_prefix() {
  let options = StyleXStateOptions::default();

  assert_eq!(options.as_str_value(), None);
  assert!(!options.is_proxy());
  assert_eq!(options.as_proxy_string(), None);
  assert_eq!(options.first_css_key(), None);
  assert_eq!(options.class_name_prefix(), Some("x"));
}

/// `is_proxy` must agree with `as_proxy_string` for every prefix: neither
/// depends on the options' contents, and a disagreement would let the
/// resolvability check and the resolution itself reach opposite conclusions.
#[test]
fn is_proxy_agrees_with_as_proxy_string() {
  for prefix in ["x", "", "custom"] {
    let options = StyleXStateOptions::default().with_class_name_prefix(prefix);

    assert_eq!(options.is_proxy(), options.as_proxy_string().is_some());
  }
}

/// An explicitly empty prefix is reported as `Some("")` rather than `None`:
/// JavaScript's `classNamePrefix != null` test is what decides whether the
/// default marker keeps its `-` separator, and an empty prefix passes it. This
/// is what makes an empty prefix yield `-default-marker`.
#[test]
fn an_empty_prefix_is_reported_as_present() {
  let options = StyleXStateOptions::default().with_class_name_prefix("");

  assert_eq!(options.class_name_prefix(), Some(""));
}
