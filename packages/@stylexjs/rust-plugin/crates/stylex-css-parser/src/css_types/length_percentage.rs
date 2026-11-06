/*!
CSS Length-Percentage union type parsing.

Handles values that can be either length or percentage values.
*/

use crate::{
  css_types::{Length, Percentage, calc::Calc},
  token_parser::TokenParser,
};
use std::fmt::{self, Display};

/// Union type for length or percentage values
#[derive(Debug, Clone, PartialEq)]
pub enum LengthPercentage {
  Length(Length),
  Percentage(Percentage),
  Calc(Calc),
}

impl LengthPercentage {
  /// Parser for length or percentage values
  pub fn parser() -> TokenParser<LengthPercentage> {
    TokenParser::one_of(vec![
      Calc::parser().map(LengthPercentage::Calc, Some("calc")),
      Percentage::parser().map(LengthPercentage::Percentage, Some("percentage")),
      Length::parser().map(LengthPercentage::Length, Some("length")),
    ])
  }

  /// Check if this is a length value
  pub fn is_length(&self) -> bool {
    matches!(self, LengthPercentage::Length(_))
  }

  /// Check if this is a percentage value
  pub fn is_percentage(&self) -> bool {
    matches!(self, LengthPercentage::Percentage(_))
  }

  /// Check if this is a calc value
  pub fn is_calc(&self) -> bool {
    matches!(self, LengthPercentage::Calc(_))
  }

  /// Get the length value if this is a length, None otherwise
  pub fn as_length(&self) -> Option<&Length> {
    match self {
      LengthPercentage::Length(length) => Some(length),
      _ => None,
    }
  }

  /// Get the percentage value if this is a percentage, None otherwise
  pub fn as_percentage(&self) -> Option<&Percentage> {
    match self {
      LengthPercentage::Percentage(percentage) => Some(percentage),
      _ => None,
    }
  }

  /// Get the calc value if this is a calc, None otherwise
  pub fn as_calc(&self) -> Option<&Calc> {
    match self {
      LengthPercentage::Calc(calc) => Some(calc),
      _ => None,
    }
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for LengthPercentage {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      LengthPercentage::Length(length) => length.fmt(f),
      LengthPercentage::Percentage(percentage) => percentage.fmt(f),
      LengthPercentage::Calc(calc) => calc.fmt(f),
    }
  }
}

/// Convenience function for creating the parser
pub fn length_percentage_parser() -> TokenParser<LengthPercentage> {
  LengthPercentage::parser()
}

#[cfg(test)]
#[path = "../tests/css_types/length_percentage_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/css_types/length_percentage_test.rs"]
mod length_percentage_test;

#[cfg(test)]
#[path = "../tests/css_types/length_percentage_coverage_test.rs"]
mod length_percentage_coverage_test;
