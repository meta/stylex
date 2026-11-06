/*!
Token types and tokenization utilities for CSS parsing.
*/

use crate::CssResult;
use cssparser::{Parser, Token as CssToken};
use log::error;
use stylex_macros::stylex_panic;
use stylex_utils::number::to_js_string;

/// Simple token representation
#[derive(Debug, Clone, PartialEq)]
pub enum SimpleToken {
  Ident(String),
  AtKeyword(String),
  Hash(String),
  String(String),
  Number(f64),
  Dimension { value: f64, unit: String },
  Percentage(f64),
  Url(String),
  Function(String),
  Delim(char),
  LeftParen,
  RightParen,
  LeftBracket,
  RightBracket,
  LeftBrace,
  RightBrace,
  Comma,
  Semicolon,
  Colon,
  Whitespace,
  Comment(String),
  Unknown(String),
}

impl SimpleToken {
  /// Extract token value
  pub fn extract_value(&self) -> Option<String> {
    match self {
      SimpleToken::Function(name) => Some(name.clone()),
      SimpleToken::Ident(value) => Some(value.clone()),
      SimpleToken::String(value) => Some(value.clone()),
      SimpleToken::Hash(value) => Some(value.clone()),
      SimpleToken::AtKeyword(value) => Some(value.clone()),
      SimpleToken::Comment(value) => Some(value.clone()),
      // Through `to_js_string`, so exponents are spelled as JavaScript spells them.
      SimpleToken::Number(value) => Some(to_js_string(*value)),
      SimpleToken::Percentage(value) => Some(to_js_string(*value)),
      SimpleToken::Dimension { value, unit } => Some(format!("{}{}", to_js_string(*value), unit)),
      SimpleToken::Delim(ch) => Some(ch.to_string()),
      SimpleToken::Unknown(value) => Some(value.clone()),
      _ => None, // No extractable value for structural tokens
    }
  }

  /// Extract numeric value for Number and Percentage tokens
  pub fn extract_number(&self) -> Option<f64> {
    match self {
      SimpleToken::Number(value) => Some(*value),
      SimpleToken::Percentage(value) => Some(*value),
      SimpleToken::Dimension { value, .. } => Some(*value),
      _ => None,
    }
  }
}

/// The number at the start of `text`, re-read at `f64` (`cssparser` stores `f32`, so `1.2`
/// would widen to `1.2000000476837158`). `None` if there is no number; callers fall back.
pub(crate) fn leading_f64(text: &str) -> Option<f64> {
  let end = leading_number_len(text)?;

  // `end` only advances over ASCII bytes, so this is a char boundary; `unwrap_or` avoids a panic.
  text.get(..end).unwrap_or("").parse::<f64>().ok()
}

/// Length in bytes of the numeric literal at the start of `text`, or `None` if there is none.
fn leading_number_len(text: &str) -> Option<usize> {
  let bytes = text.as_bytes();
  let mut end = 0;
  let mut digits = false;

  if matches!(bytes.first(), Some(b'+' | b'-')) {
    end = 1;
  }

  while matches!(bytes.get(end), Some(b'0'..=b'9')) {
    end += 1;
    digits = true;
  }

  if bytes.get(end) == Some(&b'.') {
    let mut after = end + 1;
    while matches!(bytes.get(after), Some(b'0'..=b'9')) {
      after += 1;
      digits = true;
    }
    // A trailing `.` with no digits after it is not part of the number.
    if after > end + 1 {
      end = after;
    }
  }

  if !digits {
    return None;
  }

  // An exponent only counts when complete: `1e` is `1` followed by the identifier `e`.
  if matches!(bytes.get(end), Some(b'e' | b'E')) {
    let mut after = end + 1;
    if matches!(bytes.get(after), Some(b'+' | b'-')) {
      after += 1;
    }
    let exponent_start = after;
    while matches!(bytes.get(after), Some(b'0'..=b'9')) {
      after += 1;
    }
    if after > exponent_start {
      end = after;
    }
  }

  Some(end)
}

// Total: unknown tokens map to `SimpleToken::Unknown`. `text` runs from the token's first byte,
// where numeric arms re-read the authored digits; their `unwrap_or` fallbacks are unreachable
// and keep `cssparser`'s value. Do not make the fallback cleverer.
fn map_css_token(token: &CssToken, text: &str) -> SimpleToken {
  use SimpleToken as T;
  match token {
    CssToken::Ident(v) => T::Ident(v.as_ref().to_string()),
    CssToken::AtKeyword(v) => T::AtKeyword(v.as_ref().to_string()),
    CssToken::IDHash(v) | CssToken::Hash(v) => T::Hash(v.as_ref().to_string()),
    CssToken::QuotedString(v) => T::String(v.as_ref().to_string()),
    CssToken::Number { value, .. } => T::Number(leading_f64(text).unwrap_or(*value as f64)),
    // The authored percent (`50%` is `50`), not `cssparser`'s `unit_value` fraction.
    CssToken::Percentage { unit_value, .. } => {
      T::Percentage(leading_f64(text).unwrap_or(*unit_value as f64 * 100.0))
    },
    CssToken::Dimension { value, unit, .. } => T::Dimension {
      value: leading_f64(text).unwrap_or(*value as f64),
      unit: unit.as_ref().to_string(),
    },
    CssToken::Function(v) => T::Function(v.as_ref().to_string()),
    // Map parenthesis via Delim tokens if present
    CssToken::Delim('(') => T::LeftParen,
    CssToken::Delim(')') => T::RightParen,
    CssToken::Delim(c) => T::Delim(*c),
    CssToken::WhiteSpace(_) => T::Whitespace,
    CssToken::Comma => T::Comma,
    CssToken::Colon => T::Colon,
    CssToken::Semicolon => T::Semicolon,
    CssToken::BadUrl(_) | CssToken::BadString(_) => T::Unknown(format!("{:?}", token)),
    CssToken::UnquotedUrl(url) => T::String(url.as_ref().to_string()),
    CssToken::CloseParenthesis => T::RightParen,
    CssToken::SquareBracketBlock => T::Delim('['),
    CssToken::CloseSquareBracket => T::Delim(']'),
    CssToken::CurlyBracketBlock => T::Delim('{'),
    CssToken::CloseCurlyBracket => T::Delim('}'),
    CssToken::CDC => T::Delim('>'), // --> CSS comment close
    CssToken::CDO => T::Delim('<'), // <!-- CSS comment open

    // Remaining tokens (e.g. future cssparser additions)
    _ => T::Unknown(format!("{:?}", token)),
  }
}

/// Descend into a block/function with `parse_nested_block`, panicking if the nested parse fails.
fn parse_nested_or_panic<'i, F>(parser: &mut Parser<'i>, parse: F)
where
  F: FnOnce(&mut Parser<'i>) -> Result<(), cssparser::ParseError<()>>,
{
  // Error handling lives in a non-generic helper so it monomorphizes once (coverage).
  handle_nested_block_result(parser.parse_nested_block(parse));
}

/// Panic (with diagnostics) when a nested `cssparser` block failed to parse.
fn handle_nested_block_result(result: Result<(), cssparser::ParseError<()>>) {
  if let Err(e) = result {
    error!("Error parsing nested content: {:?}", e);
    stylex_panic!("Error parsing nested content: {:?}", e); // Exit on error
  }
}

/// The byte offset in the original input where the next token starts; read before consuming.
fn next_token_offset(parser: &Parser) -> usize {
  parser.position().byte_index()
}

/// The input from `start` onward, or `""` if `start` is not a char boundary (a slice panic
/// would abort the NAPI host).
fn from_offset(input: &str, start: usize) -> &str {
  input.get(start..).unwrap_or_default()
}

/// Recursively tokenize nested content (parenthesis blocks and functions).
fn tokenize_nested_content(input: &str, parser: &mut Parser, tokens: &mut Vec<SimpleToken>) {
  let mut start = next_token_offset(parser);
  while let Ok(inner_token) = parser.next_including_whitespace_and_comments() {
    match &inner_token {
      CssToken::ParenthesisBlock => {
        tokens.push(SimpleToken::LeftParen);

        parse_nested_or_panic(parser, |nested_parser| {
          tokenize_nested_content(input, nested_parser, tokens);
          Ok(())
        });

        tokens.push(SimpleToken::RightParen);
      },
      CssToken::Function(func_name) => {
        tokens.push(SimpleToken::Function(func_name.as_ref().to_string()));

        parse_nested_or_panic(parser, |nested_parser| {
          tokenize_nested_content(input, nested_parser, tokens);
          Ok(())
        });

        tokens.push(SimpleToken::RightParen);
      },
      _ => {
        tokens.push(map_css_token(inner_token, from_offset(input, start)));
      },
    }
    start = next_token_offset(parser);
  }
}

fn tokenize_all(input: &str) -> Vec<SimpleToken> {
  let mut parser = Parser::new(input);
  // `cssparser` 0.38 refuses blocks nested past 75 levels; upstream has no limit (0 = none).
  parser.set_nested_block_limit(0);

  let mut tokens = Vec::new();
  let mut start = next_token_offset(&parser);
  while let Ok(t) = parser.next_including_whitespace_and_comments() {
    match &t {
      CssToken::Function(func_name) => {
        tokens.push(SimpleToken::Function(func_name.as_ref().to_string()));

        parse_nested_or_panic(&mut parser, |nested_parser| {
          tokenize_nested_content(input, nested_parser, &mut tokens);
          Ok(())
        });

        // cssparser consumes the closing paren itself
        tokens.push(SimpleToken::RightParen);
      },
      CssToken::ParenthesisBlock => {
        tokens.push(SimpleToken::LeftParen);

        parse_nested_or_panic(&mut parser, |nested_parser| {
          tokenize_nested_content(input, nested_parser, &mut tokens);
          Ok(())
        });

        // cssparser consumes the closing paren itself
        tokens.push(SimpleToken::RightParen);
      },
      _ => {
        tokens.push(map_css_token(t, from_offset(input, start)));
      },
    }
    start = next_token_offset(&parser);
  }
  tokens
}

/// A list of CSS tokens with parsing state
#[derive(Default)]
pub struct TokenList {
  pub tokens: Vec<SimpleToken>, // Made public for debugging
  pub current_index: usize,
  /// Parser frames open on the current path. See [`TokenList::with_depth`].
  pub(crate) depth: usize,
}

impl TokenList {
  /// Create a new TokenList from a CSS string
  pub fn new(input: &str) -> Self {
    Self {
      tokens: tokenize_all(input),
      current_index: 0,
      depth: 0,
    }
  }

  /// Run `parse` one frame deeper, refusing past the nesting budget (a stack overflow would
  /// abort the process). Counts parser frames, which bare `not` chains add without parens.
  pub(crate) fn with_depth<T, F>(&mut self, parse: F) -> CssResult<T>
  where
    F: FnOnce(&mut Self) -> CssResult<T>,
  {
    // `depth + 1`: the budget counts levels; N levels use N-1 frames (64 accepted, 65 refused).
    if self.depth + 1 >= stylex_utils::nesting::MAX_NESTING_DEPTH {
      return Err(crate::CssParseError::ParseError {
        message: format!(
          "Nesting is deeper than the {} levels the compiler parses",
          stylex_utils::nesting::MAX_NESTING_DEPTH
        ),
      });
    }

    self.depth += 1;
    let result = parse(self);
    self.depth -= 1;

    result
  }

  /// Consume the next token
  pub fn consume_next_token(&mut self) -> CssResult<Option<SimpleToken>> {
    if self.current_index < self.tokens.len() {
      let token = self.tokens[self.current_index].clone();
      self.current_index += 1;
      Ok(Some(token))
    } else {
      Ok(None)
    }
  }

  /// Consume the next token; infallible, since the list is in memory.
  pub fn consume_next_token_infallible(&mut self) -> Option<SimpleToken> {
    self.consume_next_token().ok().flatten()
  }

  /// Peek at the next token without consuming it, returning `None` at end of input.
  pub fn peek_infallible(&mut self) -> Option<SimpleToken> {
    self.peek().ok().flatten()
  }

  /// Peek at the next token without consuming it
  pub fn peek(&mut self) -> CssResult<Option<SimpleToken>> {
    if self.current_index < self.tokens.len() {
      Ok(Some(self.tokens[self.current_index].clone()))
    } else {
      Ok(None)
    }
  }

  /// Save the current position for potential rollback
  pub fn save_position(&self) -> usize {
    self.current_index
  }

  /// Restore to a previously saved position
  pub fn restore_position(&mut self, position: usize) -> CssResult<()> {
    if position <= self.tokens.len() {
      self.current_index = position;
      Ok(())
    } else {
      Err(crate::CssParseError::ParseError {
        message: "Invalid position for restore".to_string(),
      })
    }
  }

  /// Get the first token (alias for peek)
  pub fn first(&mut self) -> CssResult<Option<SimpleToken>> {
    self.peek()
  }

  /// Set the current parsing index
  pub fn set_current_index(&mut self, new_index: usize) {
    self.current_index = new_index.min(self.tokens.len());
  }

  /// Rewind the parser by a number of positions
  pub fn rewind(&mut self, positions: usize) {
    self.current_index = self.current_index.saturating_sub(positions);
  }

  /// Check if the token list is empty
  pub fn is_empty(&self) -> bool {
    self.current_index >= self.tokens.len()
  }

  /// Get all tokens
  pub fn get_all_tokens(&mut self) -> Vec<SimpleToken> {
    self.tokens.clone()
  }

  /// Get a slice of tokens from start to end index
  pub fn slice(&mut self, start: usize, end: Option<usize>) -> Vec<SimpleToken> {
    let end = end.unwrap_or(self.current_index);
    if start >= end || start >= self.tokens.len() {
      return Vec::new();
    }
    self.tokens[start..end.min(self.tokens.len())].to_vec()
  }
}

#[cfg(test)]
#[path = "tests/token_types_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/token_types_test.rs"]
mod token_types_test;

#[cfg(test)]
#[path = "tests/token_types_coverage_test.rs"]
mod token_types_coverage_test;
