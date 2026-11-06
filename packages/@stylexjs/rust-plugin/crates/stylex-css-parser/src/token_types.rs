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
      // Through `to_js_string`, like every other number-to-text path in this
      // crate. `f64`'s own `Display` never switches to exponential form, so this
      // answered `1e21` as twenty-two digits and `1e-7` as `0.0000001`, neither
      // of which JavaScript writes -- and widening the field to a double made
      // the gap wider than it had been. Nothing in the plugin calls this today,
      // which is why nothing failed; it is `pub`, and it was contradicting the
      // rule its own crate documents.
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

/// The number a numeric token was written with, at the width JavaScript reads
/// it at.
///
/// `cssparser` stores a token's number as an `f32`, so `1.2rem` arrives here as
/// `1.2000000476837158` once widened -- and the derived bounds the media query
/// merge computes from it are wrong in their third decimal. The authored digits
/// are still in the source, so they are re-read from it: Rust's `f64` parse is
/// correctly rounded, which is the same number JavaScript's is.
///
/// `text` starts at the token but runs to the end of the input, so the number is
/// taken as a prefix. `None` means no number is there to read, and the caller
/// falls back to widening `cssparser`'s own value.
pub(crate) fn leading_f64(text: &str) -> Option<f64> {
  let end = leading_number_len(text)?;

  // `end` is counted off `text`'s own bytes and only ever advances over an
  // ASCII one, so the prefix is always a char boundary and this slice always
  // exists. `unwrap_or` states that rather than asserting it: were it ever
  // untrue, the empty string fails to parse and the answer is the same `None`
  // the caller already handles, where indexing would take the process down.
  text.get(..end).unwrap_or("").parse::<f64>().ok()
}

/// Length in bytes of the numeric literal at the start of `text`, or `None`
/// where there is no number to read.
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

  // An exponent only counts when it is complete: `1e` is the number `1`
  // followed by the identifier `e`.
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

// `map_css_token` is total: every `CssToken` maps to a `SimpleToken` (the
// wildcard arm falls back to `SimpleToken::Unknown`), so it returns the token
// directly rather than an `Option`.
//
// The three numeric arms each end in `.unwrap_or(<cssparser's own value>)`, and
// none of those fallbacks is reachable: `cssparser` produced the token from
// digits that are still in `text`, so `leading_f64` finds them. They are kept
// because the alternative is a panic in a compiler, and each is *deliberately*
// what its arm did before the digits were re-read rather than an improvement on
// it: `cssparser`'s single-precision value, and for a percentage that value
// scaled back up to the authored percent -- which is exactly what made `7%` print
// as `7.000000000000001%`.
//
// Worth being precise about the baseline, because there are two and only one is
// this. On `develop` the percentage arm was the *un-scaled* fraction, so
// `unit_value as f64 * 100.0` is the pre-`leading_f64` behaviour of this branch
// and not of the code it replaced. In a comment that closes by saying not to make
// the fallback cleverer, which "before" is meant should not be left to the reader.
//
// If one of them ever does fire, the wrong digits are the symptom to chase and
// the offset bookkeeping is the cause; do not make the fallback cleverer.
//
// `text` is the input from the token's first byte onward, which is where the
// numeric variants recover the digits `cssparser`'s `f32` dropped.
fn map_css_token(token: &CssToken, text: &str) -> SimpleToken {
  use SimpleToken as T;
  match token {
    CssToken::Ident(v) => T::Ident(v.as_ref().to_string()),
    CssToken::AtKeyword(v) => T::AtKeyword(v.as_ref().to_string()),
    CssToken::IDHash(v) | CssToken::Hash(v) => T::Hash(v.as_ref().to_string()),
    CssToken::QuotedString(v) => T::String(v.as_ref().to_string()),
    CssToken::Number { value, .. } => T::Number(leading_f64(text).unwrap_or(*value as f64)),
    // The authored percent, not `cssparser`'s `unit_value` fraction: `50%` is
    // `50`. Multiplying a fraction back up is what made `7%` print as
    // `7.000000000000001%`, and a percentage is written to be read, not
    // computed with -- the callers that want a fraction divide, which is the
    // one place the division belongs.
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

    // Remaining tokens mapped to Unknown (e.g., future cssparser additions)
    _ => T::Unknown(format!("{:?}", token)),
  }
}

/// Descend into a block/function with `parse_nested_block`, panicking if the
/// nested parse fails.
///
/// All call sites pass a closure that tokenizes the nested content and returns
/// `Ok(())`, so in normal operation this never panics. The panic is a defensive
/// guard against a malformed nested block surfacing a `cssparser` error.
fn parse_nested_or_panic<'i, F>(parser: &mut Parser<'i>, parse: F)
where
  F: FnOnce(&mut Parser<'i>) -> Result<(), cssparser::ParseError<()>>,
{
  // The error-handling branch is deliberately kept in a non-generic helper. If
  // it lived here, every monomorphization of this function (one per closure
  // type at each call site) would report the unreached arm as an uncovered
  // region — a "phantom" gap that no single instantiation covers. Routing the
  // `Result` through one non-generic function collapses that coverage into a
  // single instantiation exercised by both the success and the error path.
  handle_nested_block_result(parser.parse_nested_block(parse));
}

/// Panic (with diagnostics) when a nested `cssparser` block failed to parse.
///
/// Non-generic on purpose — see `parse_nested_or_panic`.
fn handle_nested_block_result(result: Result<(), cssparser::ParseError<()>>) {
  if let Err(e) = result {
    error!("Error parsing nested content: {:?}", e);
    stylex_panic!("Error parsing nested content: {:?}", e); // Exit on error
  }
}

/// The byte offset the next token starts at.
///
/// Read *before* the token is consumed. `SourcePosition` indexes the original
/// input even inside a nested block, and the whitespace-including variant of
/// `next` leaves the cursor on the token's first byte, so this is where the
/// numeric variants read their authored digits from.
fn next_token_offset(parser: &Parser) -> usize {
  parser.position().byte_index()
}

/// The input from `start` onward, or `""` where the string has no such
/// boundary.
///
/// `start` is a byte index `cssparser` reported for a token it just produced,
/// so it lands on a character boundary within `input` and the slice succeeds.
/// Taken fallibly all the same, because the cost of being wrong is asymmetric:
/// this crate runs inside a NAPI addon, where slicing out of range aborts the
/// process with no diagnostic at all, while `""` is a value every caller already
/// handles -- `leading_f64` reads no number from it and each numeric arm falls
/// back to `cssparser`'s own value, which is the behaviour this whole path had
/// before the digits were re-read from source.
fn from_offset(input: &str, start: usize) -> &str {
  input.get(start..).unwrap_or_default()
}

/// Recursively tokenize nested content, handling ParenthesisBlock and other
/// nested structures
fn tokenize_nested_content(input: &str, parser: &mut Parser, tokens: &mut Vec<SimpleToken>) {
  let mut start = next_token_offset(parser);
  while let Ok(inner_token) = parser.next_including_whitespace_and_comments() {
    match &inner_token {
      // Handle nested ParenthesisBlock recursively
      CssToken::ParenthesisBlock => {
        // Add opening parenthesis
        tokens.push(SimpleToken::LeftParen);

        // Parse the nested parenthesis content recursively
        parse_nested_or_panic(parser, |nested_parser| {
          tokenize_nested_content(input, nested_parser, tokens);
          Ok(())
        });

        // Add closing parenthesis
        tokens.push(SimpleToken::RightParen);
      },
      // Handle nested Function tokens
      CssToken::Function(func_name) => {
        // Add the function name token
        tokens.push(SimpleToken::Function(func_name.as_ref().to_string()));

        // Parse the function content recursively
        parse_nested_or_panic(parser, |nested_parser| {
          tokenize_nested_content(input, nested_parser, tokens);
          Ok(())
        });

        // Add closing paren token
        tokens.push(SimpleToken::RightParen);
      },
      // Handle all other tokens normally
      _ => {
        tokens.push(map_css_token(inner_token, from_offset(input, start)));
      },
    }
    start = next_token_offset(parser);
  }
}

fn tokenize_all(input: &str) -> Vec<SimpleToken> {
  let mut parser = Parser::new(input);
  // Since 0.38, `cssparser` refuses a block nested deeper than 75 levels.
  // Upstream StyleX has no such limit, and 0.37 had no limit. Thus, remove
  // the limit (0 means "no limit"). If you keep the limit,
  // `parse_nested_or_panic` stops the build on input that upstream accepts.
  parser.set_nested_block_limit(0);

  let mut tokens = Vec::new();
  let mut start = next_token_offset(&parser);
  while let Ok(t) = parser.next_including_whitespace_and_comments() {
    match &t {
      // ENHANCED: Handle Function tokens by expanding their content
      CssToken::Function(func_name) => {
        // Add the function name token first
        tokens.push(SimpleToken::Function(func_name.as_ref().to_string()));

        // Parse the function content to get individual argument tokens
        parse_nested_or_panic(&mut parser, |nested_parser| {
          // Recursively tokenize everything inside the function parentheses
          tokenize_nested_content(input, nested_parser, &mut tokens);
          Ok(())
        });

        // Add closing paren token (cssparser consumes it automatically)
        tokens.push(SimpleToken::RightParen);
      },
      // ENHANCED: Handle ParenthesisBlock tokens by expanding their content
      CssToken::ParenthesisBlock => {
        // Add opening parenthesis
        tokens.push(SimpleToken::LeftParen);

        // Parse the parenthesis content to get individual tokens
        parse_nested_or_panic(&mut parser, |nested_parser| {
          // Recursively tokenize everything inside the parentheses, handling nested
          // structures
          tokenize_nested_content(input, nested_parser, &mut tokens);
          Ok(())
        });

        // Add closing parenthesis (cssparser consumes it automatically)
        tokens.push(SimpleToken::RightParen);
      },
      // Handle all other tokens normally
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

  /// Run `parse` one frame deeper, refusing to descend past the compiler's
  /// nesting budget.
  ///
  /// Counted here, where a frame is entered, rather than off the raw text,
  /// because not every frame costs a parenthesis: a parser that recurses for
  /// its operand -- `not X`, whose operand is a whole rule -- grows the stack
  /// with nothing for a text scan to count. Counting the frames themselves also
  /// cannot be bypassed by a spelling, where a scan over undecoded text would
  /// have to over-approximate: `n\6ft` tokenizes as `Ident("not")` and recurses
  /// exactly like `not`.
  ///
  /// The budget matters because past it the process **aborts** rather than
  /// panicking -- a stack overflow is not unwindable, so the `catch_unwind`
  /// around compilation never sees it and no diagnostic is ever produced. The
  /// refusal is an ordinary parse error instead, which the caller turns into
  /// invalid-syntax.
  ///
  /// The frame is released on the way out whether `parse` succeeded or not, so
  /// an alternative that fails deep inside `one_of` does not leave the budget
  /// spent for the alternatives tried after it.
  pub(crate) fn with_depth<T, F>(&mut self, parse: F) -> CssResult<T>
  where
    F: FnOnce(&mut Self) -> CssResult<T>,
  {
    // `depth + 1` rather than `depth`, because the budget counts nesting levels
    // and this counts the frames between them: the innermost level recurses
    // into nothing, so a query nesting N levels of parentheses charges N-1
    // frames. Comparing the level keeps the budget the one the docs and the
    // tests state -- sixty-four levels accepted, sixty-five refused.
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

  /// Consume the next token from an in-memory TokenList.
  ///
  /// TokenList::consume_next_token is backed by a Vec<SimpleToken> and always
  /// returns Ok(…); the Result wrapper exists only for trait compatibility.
  /// This wrapper makes the infallibility explicit so call sites don't need a
  /// `?` operator whose Err branch can never be reached.
  pub fn consume_next_token_infallible(&mut self) -> Option<SimpleToken> {
    self.consume_next_token().ok().flatten()
  }

  /// Peek at the next token without consuming it, returning `None` at
  /// end-of-input.
  ///
  /// `TokenList::peek()` always returns `Ok(Some(...))` or `Ok(None)`.
  /// `.ok().flatten()` converts the infallible `Result` to a plain `Option`,
  /// avoiding an uncovered Err-propagation region.
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

#[cfg(test)]
#[path = "tests/token_types_precision_test.rs"]
mod token_types_precision_test;
