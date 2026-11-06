//! Tests for CSSSyntax enum: From<String> conversion and Display formatting.

use crate::css_syntax::CSSSyntax;

/// Verify every valid CSS syntax string converts to the correct enum variant.
#[test]
fn from_string_all_variants() {
  let cases = vec![
    ("<angle>", CSSSyntax::Angle),
    ("<color>", CSSSyntax::Color),
    ("<image>", CSSSyntax::Image),
    ("<integer>", CSSSyntax::Integer),
    ("<length>", CSSSyntax::Length),
    ("<lengthPercentage>", CSSSyntax::LengthPercentage),
    ("<number>", CSSSyntax::Number),
    ("<percentage>", CSSSyntax::Percentage),
    ("<resolution>", CSSSyntax::Resolution),
    ("<time>", CSSSyntax::Time),
    ("<transformFunction>", CSSSyntax::TransformFunction),
    ("<transformList>", CSSSyntax::TransformList),
    ("<url>", CSSSyntax::Url),
  ];
  for (input, expected) in cases {
    assert_eq!(CSSSyntax::from(input.to_string()), expected);
  }
}

/// Verify Display output matches the expected CSS syntax string.
#[test]
fn display_all_variants() {
  assert_eq!(CSSSyntax::Angle.to_string(), "<angle>");
  assert_eq!(CSSSyntax::Color.to_string(), "<color>");
  assert_eq!(CSSSyntax::Image.to_string(), "<image>");
  assert_eq!(CSSSyntax::Integer.to_string(), "<integer>");
  assert_eq!(CSSSyntax::Length.to_string(), "<length>");
  assert_eq!(
    CSSSyntax::LengthPercentage.to_string(),
    "<lengthPercentage>"
  );
  assert_eq!(CSSSyntax::Number.to_string(), "<number>");
  assert_eq!(CSSSyntax::Percentage.to_string(), "<percentage>");
  assert_eq!(CSSSyntax::Resolution.to_string(), "<resolution>");
  assert_eq!(CSSSyntax::Time.to_string(), "<time>");
  assert_eq!(
    CSSSyntax::TransformFunction.to_string(),
    "<transformFunction>"
  );
  assert_eq!(CSSSyntax::TransformList.to_string(), "<transformList>");
  assert_eq!(CSSSyntax::Url.to_string(), "<url>");
}

/// `AsRef<str>` must round-trip through `as_str()` for every variant. This
/// is the trait callers rely on when they want to pass a `CSSSyntax` to
/// any API that accepts `impl AsRef<str>` (e.g. when writing CSS without
/// allocating an intermediate `String`).
#[test]
fn as_ref_returns_canonical_str_for_all_variants() {
  fn take_str_ref(value: impl AsRef<str>) -> String {
    value.as_ref().to_owned()
  }

  let cases = [
    (CSSSyntax::Angle, "<angle>"),
    (CSSSyntax::Color, "<color>"),
    (CSSSyntax::Image, "<image>"),
    (CSSSyntax::Integer, "<integer>"),
    (CSSSyntax::Length, "<length>"),
    (CSSSyntax::LengthPercentage, "<lengthPercentage>"),
    (CSSSyntax::Number, "<number>"),
    (CSSSyntax::Percentage, "<percentage>"),
    (CSSSyntax::Resolution, "<resolution>"),
    (CSSSyntax::Time, "<time>"),
    (CSSSyntax::TransformFunction, "<transformFunction>"),
    (CSSSyntax::TransformList, "<transformList>"),
    (CSSSyntax::Url, "<url>"),
  ];

  for (variant, expected) in cases {
    assert_eq!(<CSSSyntax as AsRef<str>>::as_ref(&variant), expected);
    assert_eq!(take_str_ref(variant), expected);
    // `AsRef` and `as_str` must agree (zero-alloc invariant).
    assert!(std::ptr::eq(variant.as_ref(), variant.as_str()));
  }
}
