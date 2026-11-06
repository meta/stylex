/*!
CSS alpha value: a number, or a percentage divided down to one.

Not bounded to `0..=1` (matches the reference compiler), so `rgba(0, 0, 0, 2)` parses.
*/

use stylex_macros::stylex_unreachable;
use stylex_utils::number::write_js_number;

use crate::{
  CssParseError,
  token_parser::TokenParser,
  token_types::{SimpleToken, TokenList},
};
use std::fmt::{self, Display};

/// Alpha value for CSS colors
#[derive(Debug, Clone, PartialEq)]
pub struct AlphaValue {
  pub value: f64, // the authored alpha, unbounded -- see the module header
}

impl AlphaValue {
  /// Create a new AlphaValue
  pub fn new(value: f64) -> Self {
    Self { value }
  }

  /// Extract an `AlphaValue` from a `SimpleToken::Percentage`; other variants are unreachable.
  pub(crate) fn extract_percentage_token(token: SimpleToken) -> AlphaValue {
    if let SimpleToken::Percentage(value) = token {
      // The token carries the authored percent: `50%` becomes `0.5`.
      AlphaValue::new(value / 100.0)
    } else {
      stylex_unreachable!()
    }
  }

  /// Extract an `AlphaValue` from a `SimpleToken::Number`; other variants are unreachable.
  pub(crate) fn extract_number_token(token: SimpleToken) -> AlphaValue {
    if let SimpleToken::Number(value) = token {
      AlphaValue::new(value)
    } else {
      stylex_unreachable!()
    }
  }

  /// Parser for alpha values
  pub fn parser() -> TokenParser<AlphaValue> {
    TokenParser::one_of(vec![
      // Percentage: v[4].signCharacter === '-' ? -1 : 1) * v[4].value) / 100
      TokenParser::<SimpleToken>::token(SimpleToken::Percentage(0.0), Some("Percentage"))
        .map(Self::extract_percentage_token, Some("percentage_to_alpha")),
      // Number: (v[4].signCharacter === '-' ? -1 : 1) * v[4].value
      TokenParser::<SimpleToken>::token(SimpleToken::Number(0.0), Some("Number"))
        .map(Self::extract_number_token, Some("number_to_alpha")),
    ])
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for AlphaValue {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write_js_number(f, self.value)
  }
}

/// Alpha as a number, for color parsing.
pub fn alpha_as_number() -> TokenParser<f64> {
  AlphaValue::parser().map(|alpha| alpha.value, Some("alpha_to_number"))
}

/// Read a legacy colour's alpha from the token list, unbounded like upstream `alphaAsNumber`;
/// a percentage divides down to a fraction.
///
/// Divergence: upstream negates a signed value twice (`-0.5` reads as `+0.5`); this keeps it
/// negative. No plugin path reaches it (see *Unwitnessed parser* in `CONTEXT.md`);
/// [`crate::css_types::common_types::NumberOrPercentage`] shares the gap.
pub(crate) fn parse_alpha_token(tokens: &mut TokenList) -> Result<f64, CssParseError> {
  let token = tokens
    .consume_next_token_infallible()
    .ok_or(CssParseError::ParseError {
      message: "Expected alpha value token".to_string(),
    })?;

  match token {
    SimpleToken::Number(value) => Ok(value),
    SimpleToken::Percentage(value) => Ok(value / 100.0),
    _ => Err(CssParseError::ParseError {
      message: format!(
        "Expected Number or Percentage token for alpha, got {:?}",
        token
      ),
    }),
  }
}

#[cfg(test)]
#[path = "../tests/css_types/alpha_value_coverage_test.rs"]
mod alpha_value_coverage_test;
