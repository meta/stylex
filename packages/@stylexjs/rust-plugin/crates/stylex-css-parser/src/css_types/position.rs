use stylex_macros::stylex_unreachable;

/**
 * CSS Position Type Parser
 *
 * Provides comprehensive position parsing for CSS layout properties.
 * Covers all major CSS position parsing scenarios with Rust type safety.
 */
use crate::{
  CssParseError,
  css_types::length_percentage::{LengthPercentage, length_percentage_parser},
  token_parser::TokenParser,
  token_types::{SimpleToken, TokenList},
};
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum HorizontalKeyword {
  Left,
  Center,
  Right,
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl fmt::Display for HorizontalKeyword {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let s = match self {
      HorizontalKeyword::Left => "left",
      HorizontalKeyword::Center => "center",
      HorizontalKeyword::Right => "right",
    };
    write!(f, "{}", s)
  }
}

impl HorizontalKeyword {
  pub fn as_str(&self) -> &str {
    match self {
      HorizontalKeyword::Left => "left",
      HorizontalKeyword::Center => "center",
      HorizontalKeyword::Right => "right",
    }
  }
}

#[derive(Debug, Clone, PartialEq)]
pub enum VerticalKeyword {
  Top,
  Center,
  Bottom,
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl fmt::Display for VerticalKeyword {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let s = match self {
      VerticalKeyword::Top => "top",
      VerticalKeyword::Center => "center",
      VerticalKeyword::Bottom => "bottom",
    };
    write!(f, "{}", s)
  }
}

impl VerticalKeyword {
  pub fn as_str(&self) -> &str {
    match self {
      VerticalKeyword::Top => "top",
      VerticalKeyword::Center => "center",
      VerticalKeyword::Bottom => "bottom",
    }
  }
}

/// | LengthPercentage | HorizontalKeyword | [HorizontalKeyword, LengthPercentage]
#[derive(Debug, Clone, PartialEq)]
pub enum Horizontal {
  Length(LengthPercentage),
  Keyword(HorizontalKeyword),
  KeywordWithOffset(HorizontalKeyword, LengthPercentage), // [HorizontalKeyword, LengthPercentage]
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl fmt::Display for Horizontal {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Horizontal::Length(lp) => write!(f, "{}", lp),
      Horizontal::Keyword(k) => write!(f, "{}", k),
      Horizontal::KeywordWithOffset(k, lp) => write!(f, "{} {}", k, lp),
    }
  }
}

/// | LengthPercentage | VerticalKeyword | [VerticalKeyword, LengthPercentage]
#[derive(Debug, Clone, PartialEq)]
pub enum Vertical {
  Length(LengthPercentage),
  Keyword(VerticalKeyword),
  KeywordWithOffset(VerticalKeyword, LengthPercentage), // [VerticalKeyword, LengthPercentage]
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl fmt::Display for Vertical {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Vertical::Length(lp) => write!(f, "{}", lp),
      Vertical::Keyword(k) => write!(f, "{}", k),
      Vertical::KeywordWithOffset(k, lp) => write!(f, "{} {}", k, lp),
    }
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Position {
  pub horizontal: Option<Horizontal>,
  pub vertical: Option<Vertical>,
}

impl Position {
  /// Create a new Position
  pub fn new(horizontal: Option<Horizontal>, vertical: Option<Vertical>) -> Self {
    Self {
      horizontal,
      vertical,
    }
  }

  fn is_horizontal_ident(token: &SimpleToken) -> bool {
    if let SimpleToken::Ident(value) = token {
      matches!(value.as_str(), "left" | "center" | "right")
    } else {
      false
    }
  }

  fn token_to_horizontal_keyword(token: SimpleToken) -> HorizontalKeyword {
    if let SimpleToken::Ident(value) = token {
      match value.as_str() {
        "left" => HorizontalKeyword::Left,
        "center" => HorizontalKeyword::Center,
        "right" => HorizontalKeyword::Right,
        _ => stylex_unreachable!(),
      }
    } else {
      stylex_unreachable!()
    }
  }

  fn horizontal_keyword_parser() -> TokenParser<HorizontalKeyword> {
    TokenParser::<SimpleToken>::token(SimpleToken::Ident(String::new()), Some("Ident"))
      .where_fn(Self::is_horizontal_ident, Some("horizontal_keyword"))
      .map(
        Self::token_to_horizontal_keyword,
        Some("to_horizontal_keyword"),
      )
  }

  fn is_vertical_ident(token: &SimpleToken) -> bool {
    if let SimpleToken::Ident(value) = token {
      matches!(value.as_str(), "top" | "center" | "bottom")
    } else {
      false
    }
  }

  fn token_to_vertical_keyword(token: SimpleToken) -> VerticalKeyword {
    if let SimpleToken::Ident(value) = token {
      match value.as_str() {
        "top" => VerticalKeyword::Top,
        "center" => VerticalKeyword::Center,
        "bottom" => VerticalKeyword::Bottom,
        _ => stylex_unreachable!(),
      }
    } else {
      stylex_unreachable!()
    }
  }

  fn vertical_keyword_parser() -> TokenParser<VerticalKeyword> {
    TokenParser::<SimpleToken>::token(SimpleToken::Ident(String::new()), Some("Ident"))
      .where_fn(Self::is_vertical_ident, Some("vertical_keyword"))
      .map(Self::token_to_vertical_keyword, Some("to_vertical_keyword"))
  }

  fn skip_required_whitespace(tokens: &mut TokenList) -> bool {
    let start = tokens.current_index;
    let mut consumed = false;

    while let Ok(Some(SimpleToken::Whitespace)) = tokens.peek() {
      let _ = tokens.consume_next_token();
      consumed = true;
    }

    if !consumed {
      tokens.set_current_index(start);
    }

    consumed
  }

  fn parse_length_percentage(tokens: &mut TokenList) -> Result<LengthPercentage, CssParseError> {
    (length_percentage_parser().run)(tokens)
  }

  fn parse_horizontal(tokens: &mut TokenList) -> Result<Horizontal, CssParseError> {
    let keyword = (Self::horizontal_keyword_parser().run)(tokens)?;
    let after_keyword = tokens.current_index;

    if Self::skip_required_whitespace(tokens) {
      match Self::parse_length_percentage(tokens) {
        Ok(offset) => return Ok(Horizontal::KeywordWithOffset(keyword, offset)),
        Err(_) => tokens.set_current_index(after_keyword),
      }
    }

    Ok(Horizontal::Keyword(keyword))
  }

  fn parse_vertical(tokens: &mut TokenList) -> Result<Vertical, CssParseError> {
    let keyword = (Self::vertical_keyword_parser().run)(tokens)?;
    let after_keyword = tokens.current_index;

    if Self::skip_required_whitespace(tokens) {
      match Self::parse_length_percentage(tokens) {
        Ok(offset) => return Ok(Vertical::KeywordWithOffset(keyword, offset)),
        Err(_) => tokens.set_current_index(after_keyword),
      }
    }

    Ok(Vertical::Keyword(keyword))
  }

  fn parse_both_keywords(tokens: &mut TokenList) -> Result<Position, CssParseError> {
    let start = tokens.current_index;

    if let Ok(horizontal) = Self::parse_horizontal(tokens) {
      if Self::skip_required_whitespace(tokens) {
        match Self::parse_vertical(tokens) {
          Ok(vertical) => return Ok(Position::new(Some(horizontal), Some(vertical))),
          Err(_) => tokens.set_current_index(start),
        }
      } else {
        tokens.set_current_index(start);
      }
    } else {
      tokens.set_current_index(start);
    }

    if let Ok(vertical) = Self::parse_vertical(tokens) {
      if Self::skip_required_whitespace(tokens) {
        match Self::parse_horizontal(tokens) {
          Ok(horizontal) => return Ok(Position::new(Some(horizontal), Some(vertical))),
          Err(_) => tokens.set_current_index(start),
        }
      } else {
        tokens.set_current_index(start);
      }
    } else {
      tokens.set_current_index(start);
    }

    Err(CssParseError::ParseError {
      message: "Expected horizontal and vertical position keywords".to_string(),
    })
  }

  fn parse_length_plus_vertical(tokens: &mut TokenList) -> Result<Position, CssParseError> {
    let start = tokens.current_index;
    let length = Self::parse_length_percentage(tokens)?;

    if !Self::skip_required_whitespace(tokens) {
      tokens.set_current_index(start);
      return Err(CssParseError::ParseError {
        message: "Expected whitespace after position length".to_string(),
      });
    }

    match Self::parse_vertical(tokens) {
      Ok(vertical) => Ok(Position::new(
        Some(Horizontal::Length(length)),
        Some(vertical),
      )),
      Err(error) => {
        tokens.set_current_index(start);
        Err(error)
      },
    }
  }

  fn parse_horizontal_keyword_plus_vertical_length(
    tokens: &mut TokenList,
  ) -> Result<Position, CssParseError> {
    let start = tokens.current_index;
    let horizontal = (Self::horizontal_keyword_parser().run)(tokens)?;

    if !Self::skip_required_whitespace(tokens) {
      tokens.set_current_index(start);
      return Err(CssParseError::ParseError {
        message: "Expected whitespace after horizontal position keyword".to_string(),
      });
    }

    match Self::parse_length_percentage(tokens) {
      Ok(vertical) => Ok(Position::new(
        Some(Horizontal::Keyword(horizontal)),
        Some(Vertical::Length(vertical)),
      )),
      Err(error) => {
        tokens.set_current_index(start);
        Err(error)
      },
    }
  }

  fn parse_vertical_keyword_plus_horizontal_length(
    tokens: &mut TokenList,
  ) -> Result<Position, CssParseError> {
    let start = tokens.current_index;
    let vertical = (Self::vertical_keyword_parser().run)(tokens)?;

    if !Self::skip_required_whitespace(tokens) {
      tokens.set_current_index(start);
      return Err(CssParseError::ParseError {
        message: "Expected whitespace after vertical position keyword".to_string(),
      });
    }

    match Self::parse_length_percentage(tokens) {
      Ok(horizontal) => Ok(Position::new(
        Some(Horizontal::Length(horizontal)),
        Some(Vertical::Keyword(vertical)),
      )),
      Err(error) => {
        tokens.set_current_index(start);
        Err(error)
      },
    }
  }

  fn parse_numbers_only(tokens: &mut TokenList) -> Result<Position, CssParseError> {
    let first = Self::parse_length_percentage(tokens)?;
    let after_first = tokens.current_index;
    let second = if Self::skip_required_whitespace(tokens) {
      match Self::parse_length_percentage(tokens) {
        Ok(length) => length,
        Err(_) => {
          tokens.set_current_index(after_first);
          first.clone()
        },
      }
    } else {
      first.clone()
    };

    Ok(Position::new(
      Some(Horizontal::Length(first)),
      Some(Vertical::Length(second)),
    ))
  }

  fn parse_position(tokens: &mut TokenList) -> Result<Position, CssParseError> {
    let start = tokens.current_index;

    for parser in [
      Self::parse_both_keywords,
      Self::parse_horizontal_keyword_plus_vertical_length,
      Self::parse_vertical_keyword_plus_horizontal_length,
      Self::parse_length_plus_vertical,
    ] {
      match parser(tokens) {
        Ok(position) => return Ok(position),
        Err(_) => tokens.set_current_index(start),
      }
    }

    match Self::parse_horizontal(tokens) {
      Ok(horizontal) => return Ok(Position::new(Some(horizontal), None)),
      Err(_) => tokens.set_current_index(start),
    }

    match Self::parse_vertical(tokens) {
      Ok(vertical) => return Ok(Position::new(None, Some(vertical))),
      Err(_) => tokens.set_current_index(start),
    }

    Self::parse_numbers_only(tokens)
  }

  /// Covers these key scenarios:
  /// 1. Single keywords: "left", "top", "center", etc.
  /// 2. Single lengths: "50%", "10px", etc.
  /// 3. Two values: "left top", "50% 25%", "center 10px", etc.
  /// 4. Keyword with offset: "left 10px", "top 20%", etc.
  ///
  /// while being much simpler and more maintainable in Rust.
  pub fn parser() -> TokenParser<Position> {
    TokenParser::new(Self::parse_position, "position")
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl fmt::Display for Position {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let parts: Vec<String> = [
      self.horizontal.as_ref().map(|h| h.to_string()),
      self.vertical.as_ref().map(|v| v.to_string()),
    ]
    .into_iter()
    .flatten()
    .collect();

    write!(f, "{}", parts.join(" "))
  }
}

pub fn position_parser() -> TokenParser<Position> {
  Position::parser()
}

#[cfg(test)]
#[path = "../tests/css_types/position_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/css_types/position_test.rs"]
mod position_test;

#[cfg(test)]
#[path = "../tests/css_types/position_coverage_test.rs"]
mod position_coverage_test;
