/*!
CSS `<length>`: absolute, font-, viewport- and container-relative units.
*/

use stylex_utils::number::to_js_string;

use crate::{token_parser::TokenParser, token_types::SimpleToken};
use std::fmt::{self, Display};

/// Font-relative length units
pub const UNITS_BASED_ON_FONT: &[&str] = &["ch", "em", "ex", "ic", "lh", "rem", "rlh"];

pub const UNITS_BASED_ON_VIEWPORT: &[&str] = &[
  "vh", "svh", "lvh", "dvh", "vw", "svw", "lvw", "dvw", "vmin", "svmin", "lvmin", "dvmin", "vmax",
  "svmax", "lvmax", "dvmax",
];

/// Container-relative length units
pub const UNITS_BASED_ON_CONTAINER: &[&str] = &["cqw", "cqi", "cqh", "cqb", "cqmin", "cqmax"];

/// Absolute length units
pub const UNITS_BASED_ON_ABSOLUTE_UNITS: &[&str] = &["px", "cm", "mm", "in", "pt"];

/// CSS Length value with unit
#[derive(Debug, Clone, PartialEq)]
pub struct Length {
  pub value: f64,
  pub unit: String,
}

impl Length {
  /// Create a new Length value
  pub fn new(value: f64, unit: impl Into<String>) -> Self {
    Self {
      value,
      unit: unit.into(),
    }
  }

  /// All valid length units
  pub fn units() -> Vec<&'static str> {
    let mut units = Vec::new();
    units.extend_from_slice(UNITS_BASED_ON_FONT);
    units.extend_from_slice(UNITS_BASED_ON_VIEWPORT);
    units.extend_from_slice(UNITS_BASED_ON_CONTAINER);
    units.extend_from_slice(UNITS_BASED_ON_ABSOLUTE_UNITS);
    units
  }

  /// Check if a unit is a valid length unit
  pub fn is_valid_unit(unit: &str) -> bool {
    Self::units().contains(&unit)
  }

  /// The `(value, unit)` of a `Dimension` token; `None` otherwise (unreachable from the parser).
  pub(crate) fn extract_length_token(token: SimpleToken) -> Option<(f64, String)> {
    if let SimpleToken::Dimension { value, unit } = token {
      Some((value, unit))
    } else {
      None
    }
  }

  /// Whether `opt` is `Some` with a valid length unit.
  pub(crate) fn is_valid_length_opt(opt: &Option<(f64, String)>) -> bool {
    if let Some((_, unit)) = opt {
      Self::is_valid_unit(unit)
    } else {
      false
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

  /// Parser for CSS length values
  pub fn parser() -> TokenParser<Length> {
    let dimension_parser = TokenParser::<SimpleToken>::token(
      SimpleToken::Dimension {
        value: 0.0,
        unit: String::new(),
      },
      Some("Dimension"),
    )
    .map(Self::extract_length_token, Some("extract_dimension"))
    .where_fn(Self::is_valid_length_opt, Some("valid_length_unit"))
    .map(
      |opt| {
        let (value, unit) = opt.unwrap();
        Length::new(value, unit)
      },
      Some("to_length"),
    );

    // Unitless zero
    let zero_parser = TokenParser::<SimpleToken>::token(SimpleToken::Number(0.0), Some("Number"))
      .where_fn(Self::is_zero_number, Some("zero_value"))
      .map(|_| Length::new(0.0, String::new()), Some("zero_length"));

    TokenParser::one_of(vec![dimension_parser, zero_parser])
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for Length {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}{}", to_js_string(self.value), self.unit)
  }
}

#[cfg(test)]
#[path = "../tests/css_types/length_coverage_test.rs"]
mod length_coverage_test;
