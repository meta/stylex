/*!
CSS `<flex>`: grid fraction values such as `1fr`.
*/

use stylex_macros::stylex_unreachable;
use stylex_utils::number::to_js_string;

use crate::{token_parser::TokenParser, token_types::SimpleToken};
use std::fmt::{self, Display};

/// CSS flex fraction unit (e.g., 1fr)
#[derive(Debug, Clone, PartialEq)]
pub struct Flex {
  pub fraction: f64,
}

impl Flex {
  /// Create a new Flex value
  pub fn new(fraction: f64) -> Self {
    Self { fraction }
  }

  /// Whether a fraction value is valid for flex (non-negative).
  pub fn is_valid_fraction(fraction: f64) -> bool {
    fraction >= 0.0
  }

  /// Whether `token` is a `Dimension` with unit `"fr"` and a non-negative value.
  pub(crate) fn is_valid_fr_dimension(token: &SimpleToken) -> bool {
    if let SimpleToken::Dimension { value, unit } = token {
      unit == "fr" && *value >= 0.0
    } else {
      false
    }
  }

  /// Extract a `Flex` from a `SimpleToken::Dimension`; other variants are unreachable.
  pub(crate) fn extract_dimension_token(token: SimpleToken) -> Flex {
    if let SimpleToken::Dimension { value, unit: _ } = token {
      Flex::new(value)
    } else {
      stylex_unreachable!()
    }
  }

  /// Parser for flex fraction values
  pub fn parser() -> TokenParser<Flex> {
    TokenParser::<SimpleToken>::token(
      SimpleToken::Dimension {
        value: 0.0,
        unit: String::new(),
      },
      Some("Dimension"),
    )
    .where_fn(Self::is_valid_fr_dimension, Some("valid_fr_unit"))
    .map(Self::extract_dimension_token, Some("to_flex"))
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for Flex {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}fr", to_js_string(self.fraction))
  }
}

#[cfg(test)]
#[path = "../tests/css_types/flex_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/css_types/flex_test.rs"]
mod flex_test;

#[cfg(test)]
#[path = "../tests/css_types/flex_coverage_test.rs"]
mod flex_coverage_test;
