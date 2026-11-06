/*!
CSS Calc Constants type parsing.

Handles calc constants like 'pi', 'e', 'infinity', '-infinity', 'NaN'.
*/

use crate::token_parser::TokenParser;
use std::fmt::{self, Display};

/// Valid calc constants
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalcConstant {
  Pi,
  E,
  Infinity,
  NegativeInfinity,
  NaN,
}

impl CalcConstant {
  /// All valid calc constants as strings
  pub fn all_constants() -> &'static [&'static str] {
    &["pi", "e", "infinity", "-infinity", "NaN"]
  }

  /// Convert from string representation
  pub fn parse(s: &str) -> Option<CalcConstant> {
    match s {
      "pi" => Some(CalcConstant::Pi),
      "e" => Some(CalcConstant::E),
      "infinity" => Some(CalcConstant::Infinity),
      "-infinity" => Some(CalcConstant::NegativeInfinity),
      "NaN" => Some(CalcConstant::NaN),
      _ => None,
    }
  }

  /// Convert to string representation
  pub fn as_str(&self) -> &'static str {
    match self {
      CalcConstant::Pi => "pi",
      CalcConstant::E => "e",
      CalcConstant::Infinity => "infinity",
      CalcConstant::NegativeInfinity => "-infinity",
      CalcConstant::NaN => "NaN",
    }
  }

  /// Check if a string is a valid calc constant
  pub fn is_valid_constant(s: &str) -> bool {
    Self::all_constants().contains(&s)
  }

  /// Parser for calc constants
  pub fn parser() -> TokenParser<CalcConstant> {
    TokenParser::<CalcConstant>::one_of(vec![
      // Order matters - check longer strings first to avoid partial matches
      TokenParser::<String>::string("-infinity").map(
        |_| CalcConstant::NegativeInfinity,
        Some("to_negative_infinity"),
      ),
      TokenParser::<String>::string("infinity")
        .map(|_| CalcConstant::Infinity, Some("to_infinity")),
      TokenParser::<String>::string("pi").map(|_| CalcConstant::Pi, Some("to_pi")),
      TokenParser::<String>::string("e").map(|_| CalcConstant::E, Some("to_e")),
      TokenParser::<String>::string("NaN").map(|_| CalcConstant::NaN, Some("to_nan")),
    ])
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for CalcConstant {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.as_str())
  }
}

#[cfg(test)]
#[path = "../tests/css_types/calc_constant_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/css_types/calc_constant_test.rs"]
mod calc_constant_test;
