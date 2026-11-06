/*!
CSS Dimension type parsing.

Handles dimensional values that can be lengths, times, frequencies, or resolutions.
*/

use crate::{token_parser::TokenParser, token_types::SimpleToken};
use std::fmt::{self, Display};

use super::{frequency::Frequency, length::Length, resolution::Resolution, time::Time};

/// Union type for all dimensional CSS values
#[derive(Debug, Clone, PartialEq)]
pub enum Dimension {
  Length(Length),
  Time(Time),
  Frequency(Frequency),
  Resolution(Resolution),
}

impl Dimension {
  /// Check if a unit belongs to any dimension type
  #[allow(dead_code)]
  fn is_valid_dimension_unit(unit: &str) -> bool {
    Length::is_valid_unit(unit)
      || Time::is_valid_unit(unit)
      || Frequency::is_valid_unit(unit)
      || Resolution::is_valid_unit(unit)
  }

  /// Create a Dimension from value and unit
  fn from_value_and_unit(value: f64, unit: impl Into<String>) -> Option<Dimension> {
    let unit = unit.into();

    if Length::is_valid_unit(&unit) {
      Some(Dimension::Length(Length::new(value, unit)))
    } else if Time::is_valid_unit(&unit) {
      Some(Dimension::Time(Time::new(value, unit)))
    } else if Frequency::is_valid_unit(&unit) {
      Some(Dimension::Frequency(Frequency::new(value, unit)))
    } else if Resolution::is_valid_unit(&unit) {
      Some(Dimension::Resolution(Resolution::new(value, unit)))
    } else {
      None
    }
  }

  /// Extract a `Dimension` from a `SimpleToken::Dimension`, returning `None` for
  /// any other token variant.
  ///
  /// The `else` branch returning `None` cannot be reached through the public
  /// parser because `tokens::dimension()` guarantees the token is a `Dimension`
  /// variant. The named function makes that branch reachable from coverage tests
  /// without modifying existing tests.
  pub(crate) fn extract_dimension_token(token: SimpleToken) -> Option<Dimension> {
    if let SimpleToken::Dimension { value, unit } = token {
      Self::from_value_and_unit(value, unit)
    } else {
      None
    }
  }

  /// Parser for dimensional values
  pub fn parse() -> TokenParser<Dimension> {
    use crate::token_parser::tokens;

    tokens::dimension()
      .map(Self::extract_dimension_token, Some("extract_dimension"))
      .where_fn(|opt| opt.is_some(), Some("valid_dimension"))
      .map(|opt| opt.unwrap(), Some("unwrap_dimension"))
  }
}

pub fn dimension() -> TokenParser<Dimension> {
  Dimension::parse()
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for Dimension {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Dimension::Length(length) => length.fmt(f),
      Dimension::Time(time) => time.fmt(f),
      Dimension::Frequency(frequency) => frequency.fmt(f),
      Dimension::Resolution(resolution) => resolution.fmt(f),
    }
  }
}

#[cfg(test)]
#[path = "../tests/css_types/dimension_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/css_types/dimension_test.rs"]
mod dimension_test;

#[cfg(test)]
#[path = "../tests/css_types/dimension_coverage_test.rs"]
mod dimension_coverage_test;
