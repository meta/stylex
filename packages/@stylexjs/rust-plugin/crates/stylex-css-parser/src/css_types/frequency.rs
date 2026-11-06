/*!
CSS Frequency type parsing.

Handles frequency values with 'Hz' (hertz) and 'KHz' (kilohertz) units.
*/

use stylex_utils::number::to_js_string;

use crate::{token_parser::TokenParser, token_types::SimpleToken};
use std::fmt::{self, Display};

/// Valid frequency units
pub const FREQUENCY_UNITS: &[&str] = &["Hz", "KHz"];

/// CSS Frequency value with unit
#[derive(Debug, Clone, PartialEq)]
pub struct Frequency {
  pub value: f64,
  pub unit: String, // "Hz" or "KHz"
}

impl Frequency {
  /// Create a new Frequency value
  pub fn new(value: f64, unit: impl Into<String>) -> Self {
    Self {
      value,
      unit: unit.into(),
    }
  }

  /// All valid frequency units
  pub fn units() -> &'static [&'static str] {
    FREQUENCY_UNITS
  }

  /// Check if a unit is a valid frequency unit
  pub fn is_valid_unit(unit: &str) -> bool {
    FREQUENCY_UNITS.contains(&unit)
  }

  /// Extract a frequency `(value, unit)` pair from a `SimpleToken`.
  ///
  /// Returns `Some` when the token is a `Dimension` with a valid frequency unit,
  /// `None` for an unrecognised unit, and `None` for any non-`Dimension` variant
  /// (the latter branch is unreachable through the public parser, which guarantees
  /// a `Dimension` token, but the named function makes it coverable from tests).
  pub(crate) fn extract_frequency_token(token: SimpleToken) -> Option<(f64, String)> {
    if let SimpleToken::Dimension { value, unit } = token {
      if Self::is_valid_unit(&unit) {
        Some((value, unit))
      } else {
        None
      }
    } else {
      None
    }
  }

  /// Parser for CSS frequency values
  pub fn parser() -> TokenParser<Frequency> {
    TokenParser::<SimpleToken>::token(
      SimpleToken::Dimension {
        value: 0.0,
        unit: String::new(),
      },
      Some("Dimension"),
    )
    .map(
      Self::extract_frequency_token,
      Some("extract_frequency_dimension"),
    )
    .where_fn(|opt| opt.is_some(), Some("valid_frequency"))
    .map(
      |opt| {
        let (value, unit) = opt.unwrap();
        Frequency::new(value, unit)
      },
      Some("to_frequency"),
    )
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for Frequency {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if self.unit == "Hz" {
      write!(f, "{}KHz", to_js_string(self.value / 1000.0))
    } else {
      write!(f, "{}{}", to_js_string(self.value), self.unit)
    }
  }
}

#[cfg(test)]
#[path = "../tests/css_types/frequency_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/css_types/frequency_test.rs"]
mod frequency_test;

#[cfg(test)]
#[path = "../tests/css_types/frequency_coverage_test.rs"]
mod frequency_coverage_test;
