/*!
CSS Angle-Percentage union type parsing.

Handles values that can be either angle or percentage values.
*/

use crate::{
  css_types::{Angle, Percentage},
  token_parser::TokenParser,
};
use std::fmt::{self, Display};

/// Union type for angle or percentage values
#[derive(Debug, Clone, PartialEq)]
pub enum AnglePercentage {
  Angle(Angle),
  Percentage(Percentage),
}

impl AnglePercentage {
  /// Parser for angle or percentage values
  pub fn parser() -> TokenParser<AnglePercentage> {
    TokenParser::one_of(vec![
      Angle::parser().map(AnglePercentage::Angle, Some("angle")),
      Percentage::parser().map(AnglePercentage::Percentage, Some("percentage")),
    ])
  }

  /// Check if this is an angle value
  pub fn is_angle(&self) -> bool {
    matches!(self, AnglePercentage::Angle(_))
  }

  /// Check if this is a percentage value
  pub fn is_percentage(&self) -> bool {
    matches!(self, AnglePercentage::Percentage(_))
  }

  /// Get the angle value if this is an angle, None otherwise
  pub fn as_angle(&self) -> Option<&Angle> {
    match self {
      AnglePercentage::Angle(angle) => Some(angle),
      _ => None,
    }
  }

  /// Get the percentage value if this is a percentage, None otherwise
  pub fn as_percentage(&self) -> Option<&Percentage> {
    match self {
      AnglePercentage::Percentage(percentage) => Some(percentage),
      _ => None,
    }
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for AnglePercentage {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      AnglePercentage::Angle(angle) => angle.fmt(f),
      AnglePercentage::Percentage(percentage) => percentage.fmt(f),
    }
  }
}

/// Convenience function for creating the parser
pub fn angle_percentage_parser() -> TokenParser<AnglePercentage> {
  AnglePercentage::parser()
}

#[cfg(test)]
#[path = "../tests/css_types/angle_percentage_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/css_types/angle_percentage_test.rs"]
mod angle_percentage_test;
