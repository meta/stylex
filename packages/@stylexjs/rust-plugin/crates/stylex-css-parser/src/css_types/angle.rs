/*!
CSS `<angle>`: `deg`, `grad`, `rad` and `turn` units.
*/

use stylex_macros::stylex_unreachable;
use stylex_utils::number::to_js_string;

use crate::{token_parser::TokenParser, token_types::SimpleToken};
use std::fmt::{self, Display};

/// Valid angle units
pub const ANGLE_UNITS: &[&str] = &["deg", "grad", "rad", "turn"];

/// CSS Angle value with unit
#[derive(Debug, Clone, PartialEq)]
pub struct Angle {
  pub value: f64,
  pub unit: String, // "deg", "grad", "rad", or "turn"
}

impl Angle {
  /// Create a new Angle value
  pub fn new(value: f64, unit: impl Into<String>) -> Self {
    Self {
      value,
      unit: unit.into(),
    }
  }

  /// All valid angle units
  pub fn units() -> &'static [&'static str] {
    ANGLE_UNITS
  }

  /// Check if a unit is a valid angle unit
  pub fn is_valid_unit(unit: &str) -> bool {
    ANGLE_UNITS.contains(&unit)
  }

  /// Whether `token` is a `Dimension` with a valid angle unit.
  pub(crate) fn is_valid_angle_dimension(token: &SimpleToken) -> bool {
    if let SimpleToken::Dimension { unit, .. } = token {
      Self::is_valid_unit(unit)
    } else {
      false
    }
  }

  /// Extract an `Angle` from a `SimpleToken::Dimension`; other variants are unreachable.
  pub(crate) fn extract_dimension_token(token: SimpleToken) -> Angle {
    if let SimpleToken::Dimension { value, unit } = token {
      Angle::new(value, unit)
    } else {
      stylex_unreachable!()
    }
  }

  /// Whether `token` is a `Number` with value `0.0`.
  pub(crate) fn is_zero_number(token: &SimpleToken) -> bool {
    if let SimpleToken::Number(value) = token {
      *value == 0.0
    } else {
      false
    }
  }

  /// Parser for CSS angle values
  pub fn parser() -> TokenParser<Angle> {
    let dimension_parser = TokenParser::<SimpleToken>::token(
      SimpleToken::Dimension {
        value: 0.0,
        unit: String::new(),
      },
      Some("Dimension"),
    )
    .where_fn(Self::is_valid_angle_dimension, Some("valid_angle_unit"))
    .map(Self::extract_dimension_token, Some("to_angle"));

    // Unitless zero, defaulting to `deg`
    let zero_parser = TokenParser::<SimpleToken>::token(SimpleToken::Number(0.0), Some("Number"))
      .where_fn(Self::is_zero_number, Some("zero_value"))
      .map(|_| Angle::new(0.0, "deg"), Some("zero_angle"));

    TokenParser::one_of(vec![dimension_parser, zero_parser])
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for Angle {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}{}", to_js_string(self.value), self.unit)
  }
}

#[cfg(test)]
#[path = "../tests/css_types/angle_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/css_types/angle_test.rs"]
mod angle_test;

#[cfg(test)]
#[path = "../tests/css_types/angle_coverage_test.rs"]
mod angle_coverage_test;
