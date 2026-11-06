/*!
CSS Dashed Identifier type parsing.

Handles dashed identifiers that start with '--' (CSS custom properties).
*/

use stylex_macros::stylex_unreachable;

use crate::{token_parser::TokenParser, token_types::SimpleToken};
use std::fmt::{self, Display};

/// CSS Dashed Identifier for custom properties
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashedIdentifier {
  pub value: String,
}

impl DashedIdentifier {
  /// Create a new DashedIdentifier
  pub fn new(value: impl Into<String>) -> Self {
    Self {
      value: value.into(),
    }
  }

  /// Check if a string is a valid dashed identifier
  /// Must start with '--' and have at least one character after
  pub fn is_valid_dashed_ident(value: &str) -> bool {
    value.starts_with("--") && value.len() > 2
  }

  /// Parser for CSS dashed identifiers
  /// Extract the identifier string from a `SimpleToken::Ident`.
  ///
  /// Panics via `stylex_unreachable!` when given any other token variant, which
  /// cannot occur through the public parser because the `Ident` token matcher
  /// guarantees the token variant. The named function makes that defensive branch
  /// reachable from coverage tests without modifying existing tests.
  pub(crate) fn extract_ident_token(token: SimpleToken) -> String {
    if let SimpleToken::Ident(value) = token {
      value
    } else {
      stylex_unreachable!()
    }
  }

  /// Parser for CSS dashed identifiers
  pub fn parser() -> TokenParser<DashedIdentifier> {
    TokenParser::<SimpleToken>::token(SimpleToken::Ident(String::new()), Some("Ident"))
      .map(Self::extract_ident_token, Some("extract_ident"))
      .where_fn(
        |value| Self::is_valid_dashed_ident(value),
        Some("valid_dashed_ident"),
      )
      .map(DashedIdentifier::new, Some("to_dashed_identifier"))
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for DashedIdentifier {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.value)
  }
}

#[cfg(test)]
#[path = "../tests/css_types/dashed_ident_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/css_types/dashed_ident_test.rs"]
mod dashed_ident_test;

#[cfg(test)]
#[path = "../tests/css_types/dashed_ident_coverage_test.rs"]
mod dashed_ident_coverage_test;
