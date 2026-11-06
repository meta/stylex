/*!
Common CSS types and shared utilities.

This module implements the foundational types used across all CSS value parsing,
providing essential shared utilities for CSS processing.
*/

use stylex_macros::stylex_unreachable;
use stylex_utils::number::{to_js_string, write_js_number};

use crate::{
  token_parser::{TokenParser, tokens},
  token_types::SimpleToken,
};
use std::fmt::{self, Display};

/// CSS-wide keywords that can be used with any CSS property
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CssWideKeyword {
  Inherit,
  Initial,
  Unset,
  Revert,
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for CssWideKeyword {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      CssWideKeyword::Inherit => write!(f, "inherit"),
      CssWideKeyword::Initial => write!(f, "initial"),
      CssWideKeyword::Unset => write!(f, "unset"),
      CssWideKeyword::Revert => write!(f, "revert"),
    }
  }
}

impl CssWideKeyword {
  fn extract_ident(token: SimpleToken) -> String {
    if let SimpleToken::Ident(value) = token {
      value
    } else {
      stylex_unreachable!()
    }
  }

  fn ident_to_keyword(value: String) -> CssWideKeyword {
    match value.as_str() {
      "inherit" => CssWideKeyword::Inherit,
      "initial" => CssWideKeyword::Initial,
      "unset" => CssWideKeyword::Unset,
      "revert" => CssWideKeyword::Revert,
      _ => stylex_unreachable!(),
    }
  }

  /// Parser for CSS-wide keywords
  pub fn parser() -> TokenParser<CssWideKeyword> {
    tokens::ident()
      .map(Self::extract_ident, Some(".value"))
      .where_fn(
        |value| matches!(value.as_str(), "inherit" | "initial" | "unset" | "revert"),
        Some("css_wide_keyword"),
      )
      .map(Self::ident_to_keyword, Some("to_keyword"))
  }

  /// Parser specifically for 'inherit'
  pub fn inherit_parser() -> TokenParser<CssWideKeyword> {
    Self::parser().where_fn(
      |keyword| matches!(keyword, CssWideKeyword::Inherit),
      Some("inherit"),
    )
  }

  /// Parser specifically for 'initial'
  pub fn initial_parser() -> TokenParser<CssWideKeyword> {
    Self::parser().where_fn(
      |keyword| matches!(keyword, CssWideKeyword::Initial),
      Some("initial"),
    )
  }

  /// Parser specifically for 'unset'
  pub fn unset_parser() -> TokenParser<CssWideKeyword> {
    Self::parser().where_fn(
      |keyword| matches!(keyword, CssWideKeyword::Unset),
      Some("unset"),
    )
  }

  /// Parser specifically for 'revert'
  pub fn revert_parser() -> TokenParser<CssWideKeyword> {
    Self::parser().where_fn(
      |keyword| matches!(keyword, CssWideKeyword::Revert),
      Some("revert"),
    )
  }
}

/// CSS 'auto' keyword
pub fn auto_parser() -> TokenParser<String> {
  TokenParser::<String>::string("auto").map(|_| "auto".to_string(), Some("auto_keyword"))
}

/// CSS variable reference: var(--name)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CssVariable {
  pub name: String,
}

impl CssVariable {
  pub fn new(name: impl Into<String>) -> Self {
    Self { name: name.into() }
  }

  fn extract_ident_string(tok: SimpleToken) -> String {
    if let SimpleToken::Ident(s) = tok {
      s
    } else {
      String::new()
    }
  }

  /// Parser for CSS variables: var(--name)
  pub fn parser() -> TokenParser<CssVariable> {
    // var(
    let fn_var = crate::token_parser::TokenParser::<String>::fn_name("var");

    // --ident
    let dashed_ident = crate::token_parser::TokenParser::<crate::token_types::SimpleToken>::token(
      crate::token_types::SimpleToken::Ident(String::new()),
      Some("Ident"),
    )
    .map(Self::extract_ident_string, Some(".value"))
    .where_fn(|s| s.starts_with("--"), Some("starts_with_--"));

    // )
    let close_paren = crate::token_parser::TokenParser::<crate::token_types::SimpleToken>::token(
      crate::token_types::SimpleToken::RightParen,
      Some("RightParen"),
    );

    fn_var
      .flat_map(move |_| dashed_ident.clone(), Some("name"))
      .flat_map(
        move |name| close_paren.map(move |_| name.clone(), Some(")")),
        Some("close"),
      )
      .map(CssVariable::new, Some("to_css_variable"))
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for CssVariable {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "var({})", self.name)
  }
}

/// CSS percentage value
#[derive(Debug, Clone, PartialEq)]
pub struct Percentage {
  pub value: f64,
}

impl Percentage {
  pub fn new(value: f64) -> Self {
    Self { value }
  }

  fn token_to_percentage(token: SimpleToken) -> Percentage {
    if let SimpleToken::Percentage(value) = token {
      // The token already carries the authored percent: `50%` is `50`.
      Percentage::new(value)
    } else {
      stylex_unreachable!()
    }
  }

  /// Parser for percentage values
  pub fn parser() -> TokenParser<Percentage> {
    tokens::percentage().map(Self::token_to_percentage, Some("to_percentage"))
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for Percentage {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}%", to_js_string(self.value))
  }
}

/// CSS number value
#[derive(Debug, Clone, PartialEq)]
pub struct Number {
  pub value: f64,
}

impl Number {
  pub fn new(value: f64) -> Self {
    Self { value }
  }

  fn token_to_number(token: SimpleToken) -> Number {
    if let SimpleToken::Number(value) = token {
      Number::new(value)
    } else {
      stylex_unreachable!()
    }
  }

  /// Parser for number values
  pub fn parser() -> TokenParser<Number> {
    tokens::number().map(Self::token_to_number, Some("to_number"))
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for Number {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write_js_number(f, self.value)
  }
}

/// Union type for number or percentage values
#[derive(Debug, Clone, PartialEq)]
pub enum NumberOrPercentage {
  Number(Number),
  Percentage(Percentage),
}

impl NumberOrPercentage {
  /// This value as a fraction: a bare number as itself, a percentage divided
  /// by 100.
  ///
  /// A percentage token carries the percent that was authored, so every caller
  /// that wants a fraction has to divide -- and eight of them were writing the
  /// same two-arm match to do it.
  ///
  /// Parity on the percentage arm, and *not* on the number arm, for the reason
  /// [`crate::css_types::alpha_value::parse_alpha_token`] sets out at length: the
  /// reference compiler's `numberOrPercentage` multiplies an already-signed token
  /// value by its sign character and so negates twice, answering `+2` for `-2`.
  /// A negative number stays negative here. No caller in the plugin reaches
  /// either, so nothing emitted differs -- but it is a divergence rather than a
  /// port, and saying so is cheaper than the next person measuring it again.
  pub fn as_fraction(&self) -> f64 {
    match self {
      NumberOrPercentage::Number(number) => number.value,
      NumberOrPercentage::Percentage(percentage) => percentage.value / 100.0,
    }
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl Display for NumberOrPercentage {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      NumberOrPercentage::Number(n) => n.fmt(f),
      NumberOrPercentage::Percentage(p) => p.fmt(f),
    }
  }
}

/// Parser for number or percentage values
pub fn number_or_percentage_parser() -> TokenParser<NumberOrPercentage> {
  TokenParser::one_of(vec![
    Number::parser().map(NumberOrPercentage::Number, Some("number")),
    Percentage::parser().map(NumberOrPercentage::Percentage, Some("percentage")),
  ])
}

#[cfg(test)]
#[path = "../tests/css_types/common_types_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/css_types/common_types_coverage_test.rs"]
mod common_types_coverage_test;
