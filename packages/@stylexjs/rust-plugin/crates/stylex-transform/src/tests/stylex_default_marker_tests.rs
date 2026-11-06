//! The shape of the default marker, and what the class name prefix does to it.

use super::*;

/// Asserts that `options` name the marker `class_name`: two entries, the class name against
/// itself (looked up by name, so order is not asserted), and the compiled marker.
fn assert_marker_is(options: &StyleXStateOptions, class_name: &str) {
  let values = default_marker_values(options);

  assert_eq!(
    values.len(),
    2,
    "{class_name}: the marker holds the class name and `$$css`"
  );

  match values.get(class_name).map(|value| value.as_ref()) {
    Some(FlatCompiledStylesValue::String(value)) => {
      assert_eq!(value, class_name, "the class name is its own value");
    },
    other => panic!("{class_name} is not the class name: {other:?}"),
  }

  match values.get(COMPILED_KEY).map(|value| value.as_ref()) {
    Some(FlatCompiledStylesValue::Bool(true)) => {},
    other => panic!("{class_name}: `$$css` is not the compiled marker: {other:?}"),
  }
}

#[test]
fn the_default_prefix_names_the_marker_class() {
  assert_marker_is(
    &StyleXStateOptions::default().with_class_name_prefix("x"),
    "x-default-marker",
  );
}

#[test]
fn a_custom_prefix_names_the_marker_class() {
  assert_marker_is(
    &StyleXStateOptions::default().with_class_name_prefix("custom"),
    "custom-default-marker",
  );
}

/// Asked explicitly, because an unset `classNamePrefix` arrives already defaulted to `x`.
#[test]
fn an_empty_prefix_keeps_the_separator() {
  assert_marker_is(
    &StyleXStateOptions::default().with_class_name_prefix(""),
    "-default-marker",
  );
}
