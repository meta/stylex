use std::fmt;

use stylex_macros::stylex_panic;

#[derive(Debug, PartialEq, Clone, Hash, Copy)]
pub enum CSSSyntax {
  Length,
  Number,
  Percentage,
  LengthPercentage,
  Color,
  Image,
  Url,
  Integer,
  Angle,
  Time,
  Resolution,
  TransformFunction,
  TransformList,
}

impl CSSSyntax {
  /// Returns the CSS-syntax token for this variant. Allocation-free; the
  /// returned slice has `'static` lifetime so callers can use it without
  /// forcing a `.to_string()`.
  // JS-parity: stylex/packages/shared/lib/common-types.js — `CSSSyntax`
  // string literals.
  #[must_use]
  pub const fn as_str(&self) -> &'static str {
    match self {
      CSSSyntax::Angle => "<angle>",
      CSSSyntax::Color => "<color>",
      CSSSyntax::Image => "<image>",
      CSSSyntax::Integer => "<integer>",
      CSSSyntax::Length => "<length>",
      CSSSyntax::LengthPercentage => "<lengthPercentage>",
      CSSSyntax::Number => "<number>",
      CSSSyntax::Percentage => "<percentage>",
      CSSSyntax::Resolution => "<resolution>",
      CSSSyntax::Time => "<time>",
      CSSSyntax::TransformFunction => "<transformFunction>",
      CSSSyntax::TransformList => "<transformList>",
      CSSSyntax::Url => "<url>",
    }
  }
}

impl AsRef<str> for CSSSyntax {
  #[inline]
  fn as_ref(&self) -> &str {
    self.as_str()
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl fmt::Display for CSSSyntax {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str(self.as_str())
  }
}

impl From<&str> for CSSSyntax {
  fn from(value: &str) -> Self {
    match value {
      "<angle>" => CSSSyntax::Angle,
      "<color>" => CSSSyntax::Color,
      "<image>" => CSSSyntax::Image,
      "<integer>" => CSSSyntax::Integer,
      "<length>" => CSSSyntax::Length,
      "<lengthPercentage>" => CSSSyntax::LengthPercentage,
      "<number>" => CSSSyntax::Number,
      "<percentage>" => CSSSyntax::Percentage,
      "<resolution>" => CSSSyntax::Resolution,
      "<time>" => CSSSyntax::Time,
      "<transformFunction>" => CSSSyntax::TransformFunction,
      "<transformList>" => CSSSyntax::TransformList,
      "<url>" => CSSSyntax::Url,
      other => stylex_panic!(r#"CSSSyntax "{}" not found"#, other),
    }
  }
}

impl From<String> for CSSSyntax {
  #[inline]
  fn from(value: String) -> Self {
    Self::from(value.as_str())
  }
}

#[cfg(test)]
#[path = "tests/css_syntax_test.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/css_syntax_error_paths_test.rs"]
mod css_syntax_error_paths_test;
