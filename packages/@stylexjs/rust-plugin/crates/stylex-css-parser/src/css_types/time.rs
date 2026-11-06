/*!
CSS Time type parsing.

Handles time values with 's' (seconds) and 'ms' (milliseconds) units.
*/

use stylex_utils::number::to_js_string;

use crate::{token_parser::TokenParser, token_types::SimpleToken};
use std::fmt::{self, Display};

/// Valid time units
pub const TIME_UNITS: &[&str] = &["s", "ms"];

/// CSS Time value with unit
#[derive(Debug, Clone, PartialEq)]
pub struct Time {
  pub value: f64,
  pub unit: String, // "s" or "ms"
}

impl Time {
  /// Create a new Time value
  pub fn new(value: f64, unit: impl Into<String>) -> Self {
    Self {
      value,
      unit: unit.into(),
    }
  }

  /// All valid time units
  pub fn units() -> &'static [&'static str] {
    TIME_UNITS
  }

  /// Check if a unit is a valid time unit
  pub fn is_valid_unit(unit: &str) -> bool {
    TIME_UNITS.contains(&unit)
  }

  /// Extract a time `(value, unit)` pair from a `SimpleToken`.
  ///
  /// Returns `Some` when the token is a `Dimension` with a valid time unit,
  /// `None` for an unrecognised unit, and `None` for any non-`Dimension` variant
  /// (the latter branch is unreachable through the public parser, which guarantees
  /// a `Dimension` token, but the named function makes it coverable from tests).
  pub(crate) fn extract_time_token(token: SimpleToken) -> Option<(f64, String)> {
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

  /// Parser for CSS time values
  pub fn parser() -> TokenParser<Time> {
    TokenParser::<SimpleToken>::token(
      SimpleToken::Dimension {
        value: 0.0,
        unit: String::new(),
      },
      Some("Dimension"),
    )
    .map(Self::extract_time_token, Some("extract_time_dimension"))
    .where_fn(|opt| opt.is_some(), Some("valid_time"))
    .map(
      |opt| {
        let (value, unit) = opt.unwrap();
        Time::new(value, unit)
      },
      Some("to_time"),
    )
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for Time {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if self.unit == "ms" {
      write!(f, "{}s", to_js_string(self.value / 1000.0))
    } else {
      write!(f, "{}{}", to_js_string(self.value), self.unit)
    }
  }
}

#[cfg(test)]
#[path = "../tests/css_types/time_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/css_types/time_test.rs"]
mod time_test;

#[cfg(test)]
#[path = "../tests/css_types/time_coverage_test.rs"]
mod time_coverage_test;
