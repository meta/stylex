/*!
CSS `<dashed-ident>`: identifiers starting with `--`.
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

  /// Whether `value` starts with `--` and has at least one character after it.
  pub fn is_valid_dashed_ident(value: &str) -> bool {
    value.starts_with("--") && value.len() > 2
  }

  /// Extract the identifier string from a `SimpleToken::Ident`; other variants are unreachable.
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
