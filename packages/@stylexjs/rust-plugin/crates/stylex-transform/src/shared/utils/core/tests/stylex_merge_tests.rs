//! Readers the merge asks about one argument of a `stylex.props`-family call.

use super::object_has_css_marker;
use crate::tests::support::object;

/// The marker is how the merge tells a compiled style from an author's object.
#[test]
fn finds_the_marker_under_a_plain_name() {
  assert!(object_has_css_marker(&object(
    "{ color: 'xa', $$css: true }"
  )));
}

#[test]
fn finds_the_marker_under_a_string_key() {
  assert!(object_has_css_marker(&object("{ '$$css': true }")));
}

#[test]
fn finds_no_marker_where_none_is_written() {
  assert!(!object_has_css_marker(&object("{ color: 'red' }")));
  assert!(!object_has_css_marker(&object("{}")));
}

/// Compiled debug markers are string-valued; false is not a marker.
#[test]
fn finds_no_marker_under_another_value() {
  assert!(!object_has_css_marker(&object("{ $$css: false }")));
  assert!(object_has_css_marker(&object("{ $$css: 'true' }")));
  assert!(!object_has_css_marker(&object("{ $$css: 1 }")));
}

/// Even where the key would fold to the marker.
#[test]
fn finds_no_marker_under_a_computed_key() {
  assert!(!object_has_css_marker(&object("{ ['$$css']: true }")));
}

#[test]
fn finds_no_marker_in_a_property_that_is_not_a_key_value_pair() {
  assert!(!object_has_css_marker(&object("{ ...compiled }")));
  assert!(!object_has_css_marker(&object(
    "{ get $$css() { return true } }"
  )));
}
