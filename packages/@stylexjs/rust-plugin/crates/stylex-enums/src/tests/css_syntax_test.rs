//! Tests for CSSSyntax enum: From<String> conversion and Display formatting.

use crate::css_syntax::CSSSyntax;

/// `AsRef<str>` returns the same slice as `as_str()` for every variant.
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
    assert!(std::ptr::eq(variant.as_ref(), variant.as_str()));
  }
}
