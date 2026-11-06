//! Tests for the readers the merge asks about one argument of a
//! `stylex.props`-family call.

use super::object_has_css_marker;
use crate::tests::support::object;

/// A compiled style carries the marker under a plain name, which is how the
/// merge tells one from an object the author wrote.
#[test]
fn finds_the_marker_under_a_plain_name() {
  assert!(object_has_css_marker(&object(
    "{ color: 'xa', $$css: true }"
  )));
}

/// The same key written as a string names the same property.
#[test]
fn finds_the_marker_under_a_string_key() {
  assert!(object_has_css_marker(&object("{ '$$css': true }")));
}

/// The marker says the object is compiled. An object that does not carry it is
/// the author's own and is left where it stands.
#[test]
fn finds_no_marker_where_none_is_written() {
  assert!(!object_has_css_marker(&object("{ color: 'red' }")));
  assert!(!object_has_css_marker(&object("{}")));
}

/// The marker is the value `true`. Any other value is a property that happens
/// to share the name.
#[test]
fn finds_no_marker_under_another_value() {
  assert!(!object_has_css_marker(&object("{ $$css: false }")));
  assert!(!object_has_css_marker(&object("{ $$css: 'true' }")));
}

/// A key spelled in a way that names nothing at compile time is not the marker,
/// even where it would fold to it.
#[test]
fn finds_no_marker_under_a_computed_key() {
  assert!(!object_has_css_marker(&object("{ ['$$css']: true }")));
}

/// A spread and a shorthand method are not key-value pairs, so neither carries
/// the marker.
#[test]
fn finds_no_marker_in_a_property_that_is_not_a_key_value_pair() {
  assert!(!object_has_css_marker(&object("{ ...compiled }")));
  assert!(!object_has_css_marker(&object(
    "{ get $$css() { return true } }"
  )));
}
