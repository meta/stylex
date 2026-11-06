/*!
CSS Resolution type parsing.

Handles resolution values with 'dpi' (dots per inch), 'dpcm' (dots per cm), and 'dppx' (dots per px) units.
*/

use stylex_macros::stylex_unreachable;
use stylex_utils::number::to_js_string;

use crate::{token_parser::TokenParser, token_types::SimpleToken};
use std::fmt::{self, Display};

/// Valid resolution units
pub const RESOLUTION_UNITS: &[&str] = &["dpi", "dpcm", "dppx"];

/// CSS Resolution value with unit
#[derive(Debug, Clone, PartialEq)]
pub struct Resolution {
  pub value: f64,
  pub unit: String, // "dpi", "dpcm", or "dppx"
}

impl Resolution {
  /// Create a new Resolution value
  pub fn new(value: f64, unit: impl Into<String>) -> Self {
    Self {
      value,
      unit: unit.into(),
    }
  }

  /// All valid resolution units
  pub fn units() -> &'static [&'static str] {
    RESOLUTION_UNITS
  }

  /// Check if a unit is a valid resolution unit
  pub fn is_valid_unit(unit: &str) -> bool {
    RESOLUTION_UNITS.contains(&unit)
  }

  /// Return `true` when `token` is a `Dimension` with a valid resolution unit.
  ///
  /// Returns `false` for any non-`Dimension` variant. This branch is unreachable
  /// through the public parser (the combinator only yields `Dimension` tokens),
  /// but the named function makes it coverable from tests.
  pub(crate) fn is_valid_resolution_dimension(token: &SimpleToken) -> bool {
    if let SimpleToken::Dimension { unit, .. } = token {
      Self::is_valid_unit(unit)
    } else {
      false
    }
  }

  /// Extract a `Resolution` from a `SimpleToken::Dimension`.
  ///
  /// Panics via `stylex_unreachable!` for any other token variant, which cannot
  /// occur through the public parser. The named function makes that defensive
  /// branch reachable from coverage tests.
  pub(crate) fn extract_dimension_token(token: SimpleToken) -> Resolution {
    if let SimpleToken::Dimension { value, unit } = token {
      Resolution::new(value, unit)
    } else {
      stylex_unreachable!()
    }
  }

  /// Parser for CSS resolution values
  pub fn parser() -> TokenParser<Resolution> {
    TokenParser::<SimpleToken>::token(
      SimpleToken::Dimension {
        value: 0.0,
        unit: String::new(),
      },
      Some("Dimension"),
    )
    .where_fn(
      Self::is_valid_resolution_dimension,
      Some("valid_resolution_unit"),
    )
    .map(Self::extract_dimension_token, Some("to_resolution"))
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for Resolution {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}{}", to_js_string(self.value), self.unit)
  }
}

#[cfg(test)]
#[path = "../tests/css_types/resolution_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/css_types/resolution_test.rs"]
mod resolution_test;

#[cfg(test)]
#[path = "../tests/css_types/resolution_coverage_test.rs"]
mod resolution_coverage_test;
