/*!
CSS Blend Mode type parsing.

Handles blend mode values for properties like mix-blend-mode and background-blend-mode.
*/

use stylex_macros::stylex_unreachable;

use crate::{token_parser::TokenParser, token_types::SimpleToken};
use std::fmt::{self, Display};

/// CSS blend mode values
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlendMode {
  Normal,
  Multiply,
  Screen,
  Overlay,
  Darken,
  Lighten,
  ColorDodge,
  ColorBurn,
  HardLight,
  SoftLight,
  Difference,
  Exclusion,
  Hue,
  Saturation,
  Color,
  Luminosity,
}

impl BlendMode {
  /// All valid blend mode values
  pub fn all_values() -> &'static [&'static str] {
    &[
      "normal",
      "multiply",
      "screen",
      "overlay",
      "darken",
      "lighten",
      "color-dodge",
      "color-burn",
      "hard-light",
      "soft-light",
      "difference",
      "exclusion",
      "hue",
      "saturation",
      "color",
      "luminosity",
    ]
  }

  /// Convert from string representation
  pub fn parse(s: &str) -> Option<BlendMode> {
    match s {
      "normal" => Some(BlendMode::Normal),
      "multiply" => Some(BlendMode::Multiply),
      "screen" => Some(BlendMode::Screen),
      "overlay" => Some(BlendMode::Overlay),
      "darken" => Some(BlendMode::Darken),
      "lighten" => Some(BlendMode::Lighten),
      "color-dodge" => Some(BlendMode::ColorDodge),
      "color-burn" => Some(BlendMode::ColorBurn),
      "hard-light" => Some(BlendMode::HardLight),
      "soft-light" => Some(BlendMode::SoftLight),
      "difference" => Some(BlendMode::Difference),
      "exclusion" => Some(BlendMode::Exclusion),
      "hue" => Some(BlendMode::Hue),
      "saturation" => Some(BlendMode::Saturation),
      "color" => Some(BlendMode::Color),
      "luminosity" => Some(BlendMode::Luminosity),
      _ => None,
    }
  }

  /// Convert to string representation
  pub fn as_str(&self) -> &'static str {
    match self {
      BlendMode::Normal => "normal",
      BlendMode::Multiply => "multiply",
      BlendMode::Screen => "screen",
      BlendMode::Overlay => "overlay",
      BlendMode::Darken => "darken",
      BlendMode::Lighten => "lighten",
      BlendMode::ColorDodge => "color-dodge",
      BlendMode::ColorBurn => "color-burn",
      BlendMode::HardLight => "hard-light",
      BlendMode::SoftLight => "soft-light",
      BlendMode::Difference => "difference",
      BlendMode::Exclusion => "exclusion",
      BlendMode::Hue => "hue",
      BlendMode::Saturation => "saturation",
      BlendMode::Color => "color",
      BlendMode::Luminosity => "luminosity",
    }
  }

  /// Check if a string is a valid blend mode
  pub fn is_valid_blend_mode(s: &str) -> bool {
    Self::all_values().contains(&s)
  }

  /// Extract the identifier string from a `SimpleToken::Ident`.
  ///
  /// Panics via `stylex_unreachable!` when given any other token variant, which
  /// cannot occur through the public parser because `tokens::ident()` guarantees
  /// the token is an `Ident`. The named function makes that defensive branch
  /// reachable from coverage tests without modifying existing tests.
  pub(crate) fn extract_ident_token(token: SimpleToken) -> String {
    if let SimpleToken::Ident(value) = token {
      value
    } else {
      stylex_unreachable!()
    }
  }

  /// Parser for blend mode values
  pub fn parser() -> TokenParser<BlendMode> {
    use crate::token_parser::tokens;

    tokens::ident()
      .map(Self::extract_ident_token, Some("extract_ident_value"))
      .where_fn(
        |value: &String| Self::is_valid_blend_mode(value),
        Some("valid_blend_mode"),
      )
      .map(|value| Self::parse(&value).unwrap(), Some("to_blend_mode"))
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for BlendMode {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.as_str())
  }
}

#[cfg(test)]
#[path = "../tests/css_types/blend_mode_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/css_types/blend_mode_test.rs"]
mod blend_mode_test;

#[cfg(test)]
#[path = "../tests/css_types/blend_mode_coverage_test.rs"]
mod blend_mode_coverage_test;
