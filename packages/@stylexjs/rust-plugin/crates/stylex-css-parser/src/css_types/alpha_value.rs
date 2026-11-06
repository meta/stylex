/*!
CSS Alpha value parsing.

Handles alpha values for colors - a number, or a percentage that divides down to
one. Not bounded to `0..=1`: the reference compiler puts no range predicate on an
alpha, only on the channels beside it, so `rgba(0, 0, 0, 2)` parses.
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

  /// Extract an `AlphaValue` from a `SimpleToken::Percentage`.
  ///
  /// Panics via `stylex_unreachable!` for any other token variant, which cannot
  /// occur through the public parser because the token combinator guarantees a
  /// `Percentage` token. The named function makes that defensive branch reachable
  /// from coverage tests.
  pub(crate) fn extract_percentage_token(token: SimpleToken) -> AlphaValue {
    if let SimpleToken::Percentage(value) = token {
      // An alpha is a fraction, and the token carries the authored percent, so
      // this is where `50%` becomes `0.5`.
      AlphaValue::new(value / 100.0)
    } else {
      stylex_unreachable!()
    }
  }

  /// Extract an `AlphaValue` from a `SimpleToken::Number`.
  ///
  /// Panics via `stylex_unreachable!` for any other token variant, which cannot
  /// occur through the public parser because the token combinator guarantees a
  /// `Number` token. The named function makes that defensive branch reachable
  /// from coverage tests.
  pub(crate) fn extract_number_token(token: SimpleToken) -> AlphaValue {
    if let SimpleToken::Number(value) = token {
      // Handle sign and use directly as alpha value
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

/// Helper function to get alpha as number for color parsing
pub fn alpha_as_number() -> TokenParser<f64> {
  AlphaValue::parser().map(|alpha| alpha.value, Some("alpha_to_number"))
}

/// Reads a legacy colour's alpha from the token list, at whatever value was
/// written.
///
/// Deliberately unbounded, which is what the reference compiler does: its
/// `alphaAsNumber` is `AlphaValue.parser.map((alpha) => alpha.value)` with no
/// `.where()` on it, while the *channels* beside it go through
/// `rgbNumberParser`, which does bound them to `0..=255`. So `rgba(0,0,0,2)`
/// parses there and has to parse here.
///
/// A percentage divides down to a fraction on the way through -- `50%` is a
/// percentage token holding `50`, and the alpha it denotes is `0.5`. That much
/// the reference compiler also does, in `AlphaValue.parser` itself.
///
/// **One thing here is not parity, and the doc used to claim it was.** The
/// reference compiler's `AlphaValue.parser` multiplies its token value by
/// `signCharacter === '-' ? -1 : 1` -- and that value already carries the sign,
/// so it negates twice. Measured through its own tokenizer rather than read off
/// the source: `-0.5` arrives as `value: -0.5` *and* `signCharacter: '-'`, so
/// the parser answers `+0.5`, and `-50%` likewise answers `+0.5`.
///
/// This reader keeps a negative alpha negative. No plugin run enters it, so no
/// emitted CSS differs and no output is evidence about it -- see the
/// *Unwitnessed parser* entry in `CONTEXT.md`. Reproducing a sign bug in a
/// reader with no caller would be the wrong way to close the gap. What is fixed
/// here is the claim: this is a divergence, and the next person to add a caller
/// needs to know that rather than trust a comment that said "parity".
///
/// [`crate::css_types::common_types::NumberOrPercentage`] inherits the same gap
/// through its number arm, and says so.
///
/// Kept as a token reader rather than a `TokenParser` because the four callers
/// read their way through a comma-separated legacy grammar by hand rather than
/// by combinator.
pub(crate) fn parse_alpha_token(tokens: &mut TokenList) -> Result<f64, CssParseError> {
  let token = tokens
    .consume_next_token_infallible()
    .ok_or(CssParseError::ParseError {
      message: "Expected alpha value token".to_string(),
    })?;

  match token {
    SimpleToken::Number(value) => Ok(value),
    // The authored percent, divided down to the fraction it denotes.
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
#[path = "../tests/css_types/alpha_value_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/css_types/alpha_value_test.rs"]
mod alpha_value_test;

#[cfg(test)]
#[path = "../tests/css_types/alpha_value_coverage_test.rs"]
mod alpha_value_coverage_test;
