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
