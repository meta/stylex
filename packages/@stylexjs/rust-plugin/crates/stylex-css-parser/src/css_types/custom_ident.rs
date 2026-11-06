/*!
CSS Custom Identifier type parsing.

Handles custom identifiers that exclude CSS reserved keywords.
*/

use stylex_macros::stylex_unreachable;

use crate::{token_parser::TokenParser, token_types::SimpleToken};
use std::fmt::{self, Display};

/// List of reserved CSS keywords that cannot be used as custom identifiers
const RESERVED_KEYWORDS: &[&str] = &[
  "unset",
  "initial",
  "inherit",
  "default",
  "none",
  "auto",
  "normal",
  "hidden",
  "visible",
  "revert",
  "revert-layer",
];

/// CSS Custom Identifier
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomIdentifier {
  pub value: String,
}

impl CustomIdentifier {
  /// Create a new CustomIdentifier
  pub fn new(value: impl Into<String>) -> Self {
    Self {
      value: value.into(),
    }
  }

  /// Check if a string is a reserved keyword
  pub fn is_reserved_keyword(value: &str) -> bool {
    RESERVED_KEYWORDS.contains(&value.to_lowercase().as_str())
  }

  /// Check if a string is a valid custom identifier
  pub fn is_valid_custom_ident(value: &str) -> bool {
    !Self::is_reserved_keyword(value) && !value.is_empty()
  }

  /// Get the list of reserved keywords
  pub fn reserved_keywords() -> &'static [&'static str] {
    RESERVED_KEYWORDS
  }

  /// Parser for CSS custom identifiers
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

  /// Parser for CSS custom identifiers
  pub fn parser() -> TokenParser<CustomIdentifier> {
    TokenParser::<SimpleToken>::token(SimpleToken::Ident(String::new()), Some("Ident"))
      .map(Self::extract_ident_token, Some("extract_ident"))
      .where_fn(
        |value| Self::is_valid_custom_ident(value),
        Some("valid_custom_ident"),
      )
      .map(CustomIdentifier::new, Some("to_custom_identifier"))
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for CustomIdentifier {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.value)
  }
}

#[cfg(test)]
#[path = "../tests/css_types/custom_ident_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/css_types/custom_ident_test.rs"]
mod custom_ident_test;

#[cfg(test)]
#[path = "../tests/css_types/custom_ident_coverage_test.rs"]
mod custom_ident_coverage_test;
