/*!
Monadic token parser combinators for building CSS parsers.
*/

use log::debug;
use rustc_hash::FxHashSet;
use stylex_macros::stylex_unreachable;

use crate::{
  CssParseError,
  token_types::{SimpleToken, TokenList},
};
use std::{
  fmt::{Debug, Display},
  rc::Rc,
};

/// Token parsers: `tokens::ident()`, `tokens::whitespace()`, etc.
pub mod tokens {
  use super::*;

  pub fn ident() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::Ident(String::new()), Some("Ident"))
  }

  pub fn whitespace() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::Whitespace, Some("Whitespace"))
  }

  pub fn comma() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::Comma, Some("Comma"))
  }

  pub fn colon() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::Colon, Some("Colon"))
  }

  pub fn semicolon() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::Semicolon, Some("Semicolon"))
  }

  pub fn number() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::Number(0.0), Some("Number"))
  }

  pub fn percentage() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::Percentage(0.0), Some("Percentage"))
  }

  pub fn dimension() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(
      SimpleToken::Dimension {
        value: 0.0,
        unit: String::new(),
      },
      Some("Dimension"),
    )
  }

  pub fn function() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::Function(String::new()), Some("Function"))
  }

  pub fn string() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::String(String::new()), Some("String"))
  }

  pub fn hash() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::Hash(String::new()), Some("Hash"))
  }

  pub fn url() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::Url(String::new()), Some("URL"))
  }

  pub fn open_paren() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::LeftParen, Some("OpenParen"))
  }

  pub fn close_paren() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::RightParen, Some("CloseParen"))
  }

  pub fn open_square() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::LeftBracket, Some("OpenSquare"))
  }

  pub fn close_square() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::RightBracket, Some("CloseSquare"))
  }

  pub fn open_curly() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::LeftBrace, Some("OpenCurly"))
  }

  pub fn close_curly() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::RightBrace, Some("CloseCurly"))
  }

  pub fn delim(ch: char) -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::Delim(ch), Some("Delim"))
  }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Either<T, U> {
  Left(T),
  Right(U),
}

/// A parser function that takes a TokenList and returns T or Error
pub type RunFn<T> = Rc<dyn Fn(&mut TokenList) -> Result<T, CssParseError>>;

#[derive(Clone)]
pub struct TokenParser<T: Clone + Debug> {
  pub run: RunFn<T>,
  pub label: String,
}

impl<T: Clone + Debug + 'static> TokenParser<T> {
  /// Create a new TokenParser
  pub fn new<F>(parser_fn: F, label: impl Into<String>) -> Self
  where
    F: Fn(&mut TokenList) -> Result<T, CssParseError> + 'static,
  {
    Self {
      run: Rc::new(parser_fn),
      label: label.into(),
    }
  }

  /// Parse a CSS string using this parser
  pub fn parse(&self, css: &str) -> Result<T, CssParseError> {
    let mut tokens = TokenList::new(css);
    (self.run)(&mut tokens)
  }

  /// Parse a CSS string and ensure all input is consumed
  pub fn parse_to_end(&self, css: &str) -> Result<T, CssParseError> {
    let mut tokens = TokenList::new(css);
    let initial_index = tokens.current_index;

    let output = (self.run)(&mut tokens);

    // Error decisions live in non-generic `parse_to_end_error` so they monomorphize once.
    let parse_error = output.as_ref().err().map(ToString::to_string);
    parse_to_end_error(&mut tokens, initial_index, &self.label, parse_error).map_or(output, Err)
  }

  /// Map the output of this parser using a function.
  pub fn map<U, F>(&self, f: F, label: Option<&str>) -> TokenParser<U>
  where
    U: Clone + Debug + 'static,
    F: Fn(T) -> U + 'static,
  {
    let run_fn = self.run.clone();
    let new_label = format!("{}.map({})", self.label, label.unwrap_or(""));

    TokenParser::new(
      move |tokens| {
        let current_index = tokens.current_index;
        let r = (run_fn)(tokens);
        rewind_if_err(tokens, current_index, r.is_err());
        r.map(&f)
      },
      &new_label,
    )
  }

  /// Flat map operation for chaining parsers.
  pub fn flat_map<U, F>(&self, f: F, label: Option<&str>) -> TokenParser<U>
  where
    U: Clone + Debug + 'static,
    F: Fn(T) -> TokenParser<U> + 'static,
  {
    let run_fn = self.run.clone();
    let new_label = format!("{}.flatMap({})", self.label, label.unwrap_or(""));

    TokenParser::new(
      move |tokens| {
        let current_index = tokens.current_index;
        let r1 = (run_fn)(tokens);
        rewind_if_err(tokens, current_index, r1.is_err());
        r1.and_then(|v| {
          let r2 = (f(v).run)(tokens);
          rewind_if_err(tokens, current_index, r2.is_err());
          r2
        })
      },
      &new_label,
    )
  }

  /// Try this parser, or fall back to another parser.
  pub fn or<U>(&self, other: TokenParser<U>) -> TokenParser<Either<T, U>>
  where
    U: Clone + Debug + 'static,
  {
    let run_fn1 = self.run.clone();
    let run_fn2 = other.run.clone();
    let new_label = if other.label == "optional" {
      format!("Optional<{}>", self.label)
    } else {
      format!("OneOf<{}, {}>", self.label, other.label)
    };

    TokenParser::new(
      move |tokens| {
        let current_index = tokens.current_index;
        let r1 = (run_fn1)(tokens);
        rewind_if_err(tokens, current_index, r1.is_err());
        r1.map(Either::Left).or_else(|_| {
          let r2 = (run_fn2)(tokens);
          rewind_if_err(tokens, current_index, r2.is_err());
          r2.map(Either::Right)
        })
      },
      &new_label,
    )
  }

  /// Apply a predicate to filter the results
  pub fn where_predicate<F>(&self, predicate: F, label: Option<&str>) -> TokenParser<T>
  where
    F: Fn(&T) -> bool + 'static,
  {
    let description = label.unwrap_or("");
    self.flat_map(
      move |output| {
        if predicate(&output) {
          TokenParser::always(output)
        } else {
          TokenParser::never()
        }
      },
      Some(description),
    )
  }

  pub fn where_fn<F>(&self, predicate: F, label: Option<&str>) -> TokenParser<T>
  where
    F: Fn(&T) -> bool + 'static,
  {
    self.where_predicate(predicate, label)
  }

  /// Parse with prefix and suffix parsers.
  pub fn surrounded_by<P, S>(
    &self,
    prefix: TokenParser<P>,
    suffix: Option<TokenParser<S>>,
  ) -> TokenParser<T>
  where
    P: Clone + Debug + 'static,
    S: Clone + Debug + 'static,
  {
    match suffix {
      Some(suffix_parser) => {
        let main_run = self.run.clone();
        let prefix_run = prefix.run.clone();
        let suffix_run = suffix_parser.run.clone();
        let new_label = format!("{}.flatMap(surrounded_prefix)", prefix.label);

        TokenParser::new(
          move |tokens| {
            let current_index = tokens.current_index;
            let r_pre = (prefix_run)(tokens);
            rewind_if_err(tokens, current_index, r_pre.is_err());
            r_pre.and_then(|_| {
              let r_main = (main_run)(tokens);
              rewind_if_err(tokens, current_index, r_main.is_err());
              r_main.and_then(|value| {
                let r_suf = (suffix_run)(tokens);
                rewind_if_err(tokens, current_index, r_suf.is_err());
                r_suf.map(|_| value)
              })
            })
          },
          &new_label,
        )
      },
      None => {
        let main_run = self.run.clone();
        let prefix_run = prefix.run.clone();
        let suffix_run = prefix.run.clone();
        let new_label = format!("{}.flatMap(surrounded_prefix_same)", prefix.label);

        TokenParser::new(
          move |tokens| {
            let current_index = tokens.current_index;
            let r_pre = (prefix_run)(tokens);
            rewind_if_err(tokens, current_index, r_pre.is_err());
            r_pre.and_then(|_| {
              let r_main = (main_run)(tokens);
              rewind_if_err(tokens, current_index, r_main.is_err());
              r_main.and_then(|value| {
                let r_suf = (suffix_run)(tokens);
                rewind_if_err(tokens, current_index, r_suf.is_err());
                r_suf.map(|_| value)
              })
            })
          },
          &new_label,
        )
      },
    }
  }

  /// Skip a parser after this one
  pub fn skip<U>(&self, skip_parser: TokenParser<U>) -> TokenParser<T>
  where
    U: Clone + Debug + 'static,
  {
    self.flat_map(
      move |output| {
        let output_clone = output;
        skip_parser.map(move |_| output_clone.clone(), None)
      },
      Some("skip"),
    )
  }

  /// Add a prefix parser
  pub fn prefix<P>(&self, prefix_parser: TokenParser<P>) -> TokenParser<T>
  where
    P: Clone + Debug + 'static,
  {
    prefix_parser.flat_map(
      {
        let self_clone = self.clone();
        move |_| self_clone.clone()
      },
      Some("prefix"),
    )
  }

  /// Add a suffix parser
  pub fn suffix<S>(&self, suffix_parser: TokenParser<S>) -> TokenParser<T>
  where
    S: Clone + Debug + 'static,
  {
    self.flat_map(
      move |output| {
        let output_clone = output;
        suffix_parser.map(move |_| output_clone.clone(), None)
      },
      Some("suffix"),
    )
  }

  /// Add a separator parser - creates a SeparatedParser for fluent API
  pub fn separated_by<S: Clone + Debug + 'static>(
    &self,
    separator: TokenParser<S>,
  ) -> SeparatedParser<T, S> {
    SeparatedParser {
      parser: self.clone(),
      separator,
    }
  }

  /// Get the label of this parser
  pub fn label(&self) -> &str {
    &self.label
  }

  /// Parse and log detailed parsing information.
  pub fn debug(&self, css: &str) -> Result<T, CssParseError> {
    debug!("Parsing '{}' with parser '{}'", css, self.label);

    let mut tokens = TokenList::new(css);
    let result = (self.run)(&mut tokens);

    let err_str = result
      .as_ref()
      .err()
      .map(ToString::to_string)
      .unwrap_or_default();
    debug_log_result(result.is_ok(), &self.label, tokens.current_index, &err_str);

    result
  }

  /// Parse with detailed error context.
  pub fn parse_with_context(&self, css: &str) -> Result<T, CssParseError> {
    let mut tokens = TokenList::new(css);
    let result = (self.run)(&mut tokens);
    let err_opt = result
      .as_ref()
      .err()
      .map(|e| build_parse_with_context_error(e, css, tokens.current_index));
    err_opt.map_or(result, Err)
  }

  /// Enhanced labeling method for better debugging
  pub fn with_label(mut self, new_label: impl Into<String>) -> Self {
    self.label = new_label.into();
    self
  }

  /// Parser that always succeeds with the given value.
  pub fn always(value: T) -> TokenParser<T> {
    let label = always_make_label(std::any::type_name::<T>(), &format!("{:?}", value));
    TokenParser::new(move |_| Ok(value.clone()), &label)
  }

  /// Parser that always fails
  pub fn never() -> TokenParser<T> {
    TokenParser::new(
      |_| {
        Err(CssParseError::ParseError {
          message: "Never".to_string(),
        })
      },
      "Never",
    )
  }

  pub fn separated_by_optional_whitespace(self) -> TokenParser<Vec<T>>
  where
    T: Clone + Debug + 'static,
  {
    self
      .separated_by(tokens::whitespace().optional())
      .one_or_more()
  }

  /// Try multiple parsers in order.
  pub fn one_of(parsers: Vec<TokenParser<T>>) -> TokenParser<T> {
    TokenParser::new(
      move |tokens| {
        let mut errors = Vec::new();
        let index = tokens.current_index;

        for parser in &parsers {
          let r = (parser.run)(tokens);
          let failed = r.is_err();
          rewind_if_err(tokens, index, failed);
          if !failed {
            return r;
          }
          errors.extend(r.err());
        }

        Err(one_of_error(errors))
      },
      "oneOf",
    )
  }

  /// Parse a sequence of parsers (without separators).
  pub fn sequence<U: Clone + Debug + 'static>(parsers: Vec<TokenParser<U>>) -> TokenParser<Vec<U>> {
    TokenParser::new(
      move |tokens| {
        let current_index = tokens.current_index;
        let mut results = Vec::new();

        for parser in &parsers {
          match (parser.run)(tokens) {
            Ok(value) => results.push(value),
            Err(error) => {
              tokens.set_current_index(current_index);
              return Err(error);
            },
          }
        }

        Ok(results)
      },
      "sequence",
    )
  }

  /// Parse a sequence of parsers with separator support (returns builder)
  pub fn sequence_with_separators<U: Clone + Debug + 'static>(
    parsers: Vec<TokenParser<U>>,
  ) -> SequenceParsers<U> {
    SequenceParsers::new(parsers)
  }

  /// Sequence of required (`Left`) and optional (`Right`) parsers with separators.
  pub fn flexible_sequence_separated_by<U: Clone + Debug + 'static, S: Clone + Debug + 'static>(
    parsers: Vec<Either<TokenParser<U>, TokenParser<Option<U>>>>,
    separator: TokenParser<S>,
  ) -> TokenParser<Vec<Option<U>>> {
    TokenParser::new(
      move |tokens| {
        let current_index = tokens.current_index;
        let mut results = Vec::new();

        for (i, parser_either) in parsers.iter().enumerate() {
          if i > 0 {
            let separator_index = tokens.current_index;
            let separator_consumed = (separator.run)(tokens).is_ok();

            match parser_either {
              Either::Left(required_parser) => {
                // Required: must have a separator if not first
                if !separator_consumed && i > 0 {
                  tokens.set_current_index(current_index);
                  return Err(CssParseError::ParseError {
                    message: format!("Expected separator before required parser {}", i),
                  });
                }

                match (required_parser.run)(tokens) {
                  Ok(value) => results.push(Some(value)),
                  Err(e) => {
                    tokens.set_current_index(current_index);
                    return Err(e);
                  },
                }
              },
              Either::Right(optional_parser) => {
                match (optional_parser.run)(tokens) {
                  Ok(Some(value)) => {
                    if !separator_consumed && i > 0 {
                      tokens.set_current_index(current_index);
                      return Err(CssParseError::ParseError {
                        message: format!(
                          "Expected separator before optional parser {} that matched",
                          i
                        ),
                      });
                    }
                    results.push(Some(value));
                  },
                  Ok(None) => {
                    // None: rewind the separator if consumed
                    if separator_consumed {
                      tokens.set_current_index(separator_index);
                    }
                    results.push(None);
                  },
                  Err(_) => {
                    // Failed: rewind the separator if consumed
                    if separator_consumed {
                      tokens.set_current_index(separator_index);
                    }
                    results.push(None);
                  },
                }
              },
            }
          } else {
            match parser_either {
              Either::Left(required_parser) => match (required_parser.run)(tokens) {
                Ok(value) => results.push(Some(value)),
                Err(e) => {
                  tokens.set_current_index(current_index);
                  return Err(e);
                },
              },
              Either::Right(optional_parser) => match (optional_parser.run)(tokens) {
                Ok(option_value) => results.push(option_value),
                Err(_) => results.push(None),
              },
            }
          }
        }

        Ok(results)
      },
      "flexibleSequenceSeparatedBy",
    )
  }

  /// Parse a set of parsers in any order; chain `.separated_by()` on the result.
  pub fn set_of<U: Clone + Debug + 'static>(parsers: Vec<TokenParser<U>>) -> SetOfParsers<U> {
    SetOfParsers::new(parsers)
  }

  /// Parse zero or more occurrences.
  pub fn zero_or_more(parser: TokenParser<T>) -> TokenParser<Vec<T>> {
    let label = format!("ZeroOrMore<{}>", parser.label);
    TokenParser::new(
      move |tokens| {
        let mut results = Vec::new();
        loop {
          let current_index = tokens.current_index;
          let r = (parser.run)(tokens);
          let done = r.is_err();
          rewind_if_err(tokens, current_index, done);
          results.extend(r.ok());
          if done {
            break;
          }
        }
        Ok(results)
      },
      &label,
    )
  }

  /// Parse one or more occurrences.
  pub fn one_or_more(parser: TokenParser<T>) -> TokenParser<Vec<T>> {
    let label = format!("OneOrMore<{}>", parser.label);
    TokenParser::new(
      move |tokens| {
        let mut results = Vec::new();
        let start_index = tokens.current_index;

        let r1 = (parser.run)(tokens);
        rewind_if_err(tokens, start_index, r1.is_err());
        if r1.is_err() {
          return r1.map(|_| results);
        }
        results.extend(r1.ok());

        loop {
          let current_index = tokens.current_index;
          let r = (parser.run)(tokens);
          let done = r.is_err();
          rewind_if_err(tokens, current_index, done);
          results.extend(r.ok());
          if done {
            break;
          }
        }

        Ok(results)
      },
      &label,
    )
  }

  /// Parse a specific token type
  pub fn token(expected_token: SimpleToken, label: Option<&str>) -> TokenParser<SimpleToken> {
    let label_str = label
      .unwrap_or(&format!("{:?}", expected_token))
      .to_string();

    TokenParser::new(
      move |tokens| {
        let current_index = tokens.current_index;

        match_next_token(tokens, current_index, &expected_token)
      },
      &label_str,
    )
  }

  /// Parse a specific string as an identifier
  pub fn string(expected: &str) -> TokenParser<String> {
    let expected_clone = expected.to_string();
    Self::token(SimpleToken::Ident(String::new()), Some("Ident"))
      .map(extract_ident_value, Some(".value"))
      .where_predicate(
        move |value| value == &expected_clone,
        Some(&format!("=== {}", expected)),
      )
  }

  pub fn fn_name(name: &str) -> TokenParser<String> {
    let name_owned = name.to_string();
    Self::token(SimpleToken::Function(String::new()), Some("Function"))
      .map(extract_function_value, Some(".value"))
      .where_predicate(
        move |value| value == &name_owned,
        Some(&format!("=== {}", name)),
      )
  }

  /// Parse an identifier token
  pub fn ident() -> TokenParser<SimpleToken> {
    Self::token(SimpleToken::Ident(String::new()), Some("Ident"))
  }

  /// One or more separated by separator.
  pub fn one_or_more_separated_by<S>(
    parser: TokenParser<T>,
    separator: TokenParser<S>,
  ) -> TokenParser<Vec<T>>
  where
    S: Clone + Debug + 'static,
  {
    let label = format!(
      "OneOrMoreSeparatedBy<{}, {}>",
      parser.label, separator.label
    );
    TokenParser::new(
      move |tokens| {
        let mut results = Vec::new();
        let start_index = tokens.current_index;

        let r1 = (parser.run)(tokens);
        rewind_if_err(tokens, start_index, r1.is_err());
        if r1.is_err() {
          return r1.map(|_| results);
        }
        results.extend(r1.ok());

        loop {
          let separator_index = tokens.current_index;
          let sep_ok = (separator.run)(tokens).is_ok();
          rewind_if_err(tokens, separator_index, !sep_ok);
          if !sep_ok {
            break;
          }

          let r = (parser.run)(tokens);
          let val_failed = r.is_err();
          rewind_if_err(tokens, separator_index, val_failed);
          results.extend(r.ok());
          if val_failed {
            break;
          }
        }

        Ok(results)
      },
      &label,
    )
  }

  /// Zero or more separated by separator.
  pub fn zero_or_more_separated_by<S>(
    parser: TokenParser<T>,
    separator: TokenParser<S>,
  ) -> TokenParser<Vec<T>>
  where
    S: Clone + Debug + 'static,
  {
    let label = format!(
      "ZeroOrMoreSeparatedBy<{}, {}>",
      parser.label, separator.label
    );
    TokenParser::new(
      move |tokens| {
        let mut results = Vec::new();
        let current_index = tokens.current_index;

        let r0 = (parser.run)(tokens);
        let first_failed = r0.is_err();
        rewind_if_err(tokens, current_index, first_failed);
        if first_failed {
          return Ok(results); // Empty list is valid for zero or more
        }
        results.extend(r0.ok());

        loop {
          let separator_index = tokens.current_index;
          let sep_ok = (separator.run)(tokens).is_ok();
          rewind_if_err(tokens, separator_index, !sep_ok);
          if !sep_ok {
            break;
          }

          let r = (parser.run)(tokens);
          let val_failed = r.is_err();
          rewind_if_err(tokens, separator_index, val_failed);
          results.extend(r.ok());
          if val_failed {
            break;
          }
        }

        Ok(results)
      },
      &label,
    )
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl<T: Clone + Debug> Display for TokenParser<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.label)
  }
}

#[derive(Clone)]
pub struct TokenOptionalParser<T: Clone + Debug> {
  pub parser: TokenParser<T>,
}

impl<T: Clone + Debug + 'static> TokenOptionalParser<T> {
  pub fn new(parser: TokenParser<T>) -> Self {
    Self { parser }
  }

  /// Get the underlying parser as `TokenParser<Option<T>>`.
  pub fn as_token_parser(self) -> TokenParser<Option<T>> {
    let parser_run = self.parser.run;
    let label = format!("Optional<{}>", self.parser.label);

    TokenParser::new(
      move |tokens| {
        let current_index = tokens.current_index;
        let r = (parser_run)(tokens);
        rewind_if_err(tokens, current_index, r.is_err());
        Ok(r.ok())
      },
      &label,
    )
  }
}

impl<T: Clone + Debug + 'static> TokenParser<T> {
  /// Create a TokenOptionalParser from this parser
  pub fn optional(self) -> TokenParser<Option<T>> {
    TokenOptionalParser::new(self).as_token_parser()
  }
}

/// A parser separated by a separator parser, e.g. `parser.separated_by(comma).one_or_more()`.
#[derive(Clone)]
pub struct SeparatedParser<T: Clone + Debug + 'static, S: Clone + Debug + 'static> {
  parser: TokenParser<T>,
  separator: TokenParser<S>,
}

impl<T: Clone + Debug + 'static, S: Clone + Debug + 'static> SeparatedParser<T, S> {
  /// Parse one or more occurrences with separator
  pub fn one_or_more(self) -> TokenParser<Vec<T>> {
    TokenParser::one_or_more_separated_by(self.parser, self.separator)
  }

  /// Parse zero or more occurrences with separator
  pub fn zero_or_more(self) -> TokenParser<Vec<T>> {
    TokenParser::zero_or_more_separated_by(self.parser, self.separator)
  }

  /// Convert to a regular TokenParser (defaults to one or more)
  pub fn as_token_parser(self) -> TokenParser<Vec<T>> {
    self.one_or_more()
  }
}

#[derive(Clone)]
pub struct TokenZeroOrMoreParsers<T: Clone + Debug> {
  parser: TokenParser<T>,
  separator: Option<TokenParser<()>>,
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl<T: Clone + Debug + 'static> TokenZeroOrMoreParsers<T> {
  pub fn new(parser: TokenParser<T>, separator: Option<TokenParser<()>>) -> Self {
    Self { parser, separator }
  }

  /// Add a separator parser
  pub fn separated_by<S: Clone + Debug + 'static>(
    self,
    separator: TokenParser<S>,
  ) -> SeparatedParser<T, S> {
    SeparatedParser {
      parser: self.parser,
      separator,
    }
  }

  /// Convert to a regular TokenParser
  pub fn as_token_parser(self) -> TokenParser<Vec<T>> {
    let parser = self.parser;
    let separator = self.separator;
    let label = format!("ZeroOrMore<{}>", parser.label);

    TokenParser::new(
      move |tokens| {
        let mut results = Vec::new();

        for i in 0.. {
          if i > 0 && separator.is_some() {
            let current_index = tokens.current_index;
            if let Some(ref sep) = separator {
              match (sep.run)(tokens) {
                Ok(_) => {},
                Err(_) => {
                  tokens.set_current_index(current_index);
                  return Ok(results);
                },
              }
            }
          }

          let current_index = tokens.current_index;
          match (parser.run)(tokens) {
            Ok(value) => results.push(value),
            Err(_) => {
              tokens.set_current_index(current_index);
              return Ok(results);
            },
          }
        }

        Ok(results)
      },
      &label,
    )
  }
}

#[derive(Clone)]
pub struct TokenOneOrMoreParsers<T: Clone + Debug> {
  parser: TokenParser<T>,
  separator: Option<TokenParser<()>>,
}

#[derive(Clone)]
pub struct SetOfParsers<T: Clone + Debug> {
  parsers: Vec<TokenParser<T>>,
}

impl<T: Clone + Debug + 'static> SetOfParsers<T> {
  pub fn new(parsers: Vec<TokenParser<T>>) -> Self {
    Self { parsers }
  }

  /// Add a separator parser
  pub fn separated_by<S: Clone + Debug + 'static>(
    self,
    separator: TokenParser<S>,
  ) -> TokenParser<Vec<T>> {
    let parsers = self.parsers;
    TokenParser::new(
      move |tokens| {
        let start_index = tokens.current_index;
        let mut results = vec![None; parsers.len()];
        let mut used_indices = FxHashSet::default();
        let mut errors = Vec::new();

        for position in 0..parsers.len() {
          let mut found = false;
          let mut position_errors = Vec::new();

          // Separator between elements, not before the first
          if position > 0 {
            match (separator.run)(tokens) {
              Ok(_) => {
                // Separator consumed, continue
              },
              Err(e) => {
                // No separator found - this is an error for setOf with separators
                tokens.set_current_index(start_index);
                return Err(CssParseError::ParseError {
                  message: format!(
                    "SetOf: Expected separator before position {}: {}",
                    position, e
                  ),
                });
              },
            }
          }

          for (parser_index, parser) in parsers.iter().enumerate() {
            if used_indices.contains(&parser_index) {
              continue;
            }

            let before_attempt = tokens.current_index;
            match (parser.run)(tokens) {
              Ok(value) => {
                results[parser_index] = Some(value);
                used_indices.insert(parser_index);
                found = true;
                break;
              },
              Err(e) => {
                tokens.set_current_index(before_attempt);
                position_errors.push(format!("Parser {}: {}", parser_index, e));
              },
            }
          }

          if !found {
            errors.extend(position_errors);
            tokens.set_current_index(start_index);
            return Err(CssParseError::ParseError {
              message: format!(
                "SetOf failed at position {}: {}",
                position,
                errors.join("; ")
              ),
            });
          }
        }

        // The `Err` arm is unreachable: the loop returns early when a position is not found.
        set_of_incomplete_error(collect_set_results(results), tokens, start_index)
      },
      "setOfSeparatedBy",
    )
  }

  /// Convert to a regular TokenParser without separators
  pub fn as_token_parser(self) -> TokenParser<Vec<T>> {
    let parsers = self.parsers;
    TokenParser::new(
      move |tokens| {
        let start_index = tokens.current_index;
        let mut results = vec![None; parsers.len()];
        let mut used_indices = FxHashSet::default();
        let mut errors = Vec::new();

        for position in 0..parsers.len() {
          let mut found = false;
          let mut position_errors = Vec::new();

          for (parser_index, parser) in parsers.iter().enumerate() {
            if used_indices.contains(&parser_index) {
              continue;
            }

            let before_attempt = tokens.current_index;
            match (parser.run)(tokens) {
              Ok(value) => {
                results[parser_index] = Some(value);
                used_indices.insert(parser_index);
                found = true;
                break;
              },
              Err(e) => {
                tokens.set_current_index(before_attempt);
                position_errors.push(format!("Parser {}: {}", parser_index, e));
              },
            }
          }

          if !found {
            errors.extend(position_errors);
            tokens.set_current_index(start_index);
            return Err(CssParseError::ParseError {
              message: format!(
                "SetOf failed at position {}: {}",
                position,
                errors.join("; ")
              ),
            });
          }
        }

        // `Err` unreachable, as in the separated_by variant.
        set_of_incomplete_error(collect_set_results(results), tokens, start_index)
      },
      "setOf",
    )
  }
}

#[derive(Clone)]
pub struct SequenceParsers<T: Clone + Debug> {
  parsers: Vec<TokenParser<T>>,
}

impl<T: Clone + Debug + 'static> SequenceParsers<T> {
  pub fn new(parsers: Vec<TokenParser<T>>) -> Self {
    Self { parsers }
  }

  /// Parse a sequence without separators.
  pub fn as_token_parser(self) -> TokenParser<Vec<T>> {
    let parsers = self.parsers;
    TokenParser::new(
      move |tokens| {
        let current_index = tokens.current_index;
        let mut results = Vec::new();

        for parser in &parsers {
          match (parser.run)(tokens) {
            Ok(value) => results.push(value),
            Err(error) => {
              tokens.set_current_index(current_index);
              return Err(error);
            },
          }
        }

        Ok(results)
      },
      "sequence",
    )
  }

  /// Parse a sequence with separators; a separator is optional when the element
  /// only parses without it.
  pub fn separated_by<S: Clone + Debug + 'static>(
    self,
    separator: TokenParser<S>,
  ) -> TokenParser<Vec<T>> {
    let parsers = self.parsers;
    TokenParser::new(
      move |tokens| {
        let current_index = tokens.current_index;
        let mut results = Vec::new();

        for (i, parser) in parsers.iter().enumerate() {
          if i > 0 {
            let separator_index = tokens.current_index;

            let separator_consumed = (separator.run)(tokens).is_ok();

            match (parser.run)(tokens) {
              Ok(value) => {
                results.push(value);
                // Success - continue to next parser
              },
              Err(_) => {
                if separator_consumed {
                  // Rewind past the separator and retry without it
                  tokens.set_current_index(separator_index);
                  match (parser.run)(tokens) {
                    Ok(value) => {
                      results.push(value);
                    },
                    Err(e) => {
                      tokens.set_current_index(current_index);
                      return Err(e);
                    },
                  }
                } else {
                  tokens.set_current_index(current_index);
                  return Err(CssParseError::ParseError {
                    message: format!(
                      "SequenceSeparatedBy: Parser {} failed and no separator found",
                      i
                    ),
                  });
                }
              },
            }
          } else {
            match (parser.run)(tokens) {
              Ok(value) => results.push(value),
              Err(e) => {
                tokens.set_current_index(current_index);
                return Err(e);
              },
            }
          }
        }

        Ok(results)
      },
      "sequenceSeparatedBy",
    )
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl<T: Clone + Debug + 'static> TokenOneOrMoreParsers<T> {
  pub fn new(parser: TokenParser<T>, separator: Option<TokenParser<()>>) -> Self {
    Self { parser, separator }
  }

  /// Add a separator parser
  pub fn separated_by<S: Clone + Debug + 'static>(
    self,
    separator: TokenParser<S>,
  ) -> SeparatedParser<T, S> {
    SeparatedParser {
      parser: self.parser,
      separator,
    }
  }

  /// Convert to a regular TokenParser
  pub fn as_token_parser(self) -> TokenParser<Vec<T>> {
    let parser = self.parser;
    let separator = self.separator;
    let label = format!("OneOrMore<{}>", parser.label);

    TokenParser::new(
      move |tokens| {
        let mut results = Vec::new();

        for i in 0.. {
          if i > 0 && separator.is_some() {
            let current_index = tokens.current_index;
            if let Some(ref sep) = separator {
              match (sep.run)(tokens) {
                Ok(_) => {},
                Err(_) => {
                  tokens.set_current_index(current_index);
                  return Ok(results);
                },
              }
            }
          }

          let current_index = tokens.current_index;
          match (parser.run)(tokens) {
            Ok(value) => results.push(value),
            Err(e) => {
              if i == 0 {
                tokens.set_current_index(current_index);
                return Err(e);
              }
              return Ok(results);
            },
          }
        }

        Ok(results)
      },
      &label,
    )
  }
}

/// Tokens API providing access to all basic token parsers
pub struct Tokens;

impl Tokens {
  /// Parse an identifier token
  pub fn ident() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::Ident(String::new()), Some("Ident"))
  }

  /// Parse a comma token
  pub fn comma() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::Comma, Some("Comma"))
  }

  /// Parse a colon token
  pub fn colon() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::Colon, Some("Colon"))
  }

  /// Parse a semicolon token
  pub fn semicolon() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::Semicolon, Some("Semicolon"))
  }

  /// Parse a left parenthesis token
  pub fn open_paren() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::LeftParen, Some("OpenParen"))
  }

  /// Parse a right parenthesis token
  pub fn close_paren() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::RightParen, Some("CloseParen"))
  }

  /// Parse a left bracket token
  pub fn open_square() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::LeftBracket, Some("OpenSquare"))
  }

  /// Parse a right bracket token
  pub fn close_square() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::RightBracket, Some("CloseSquare"))
  }

  /// Parse a left brace token
  pub fn open_curly() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::LeftBrace, Some("OpenCurly"))
  }

  /// Parse a right brace token
  pub fn close_curly() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::RightBrace, Some("CloseCurly"))
  }

  /// Parse a number token
  pub fn number() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::Number(0.0), Some("Number"))
  }

  /// Parse a percentage token
  pub fn percentage() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::Percentage(0.0), Some("Percentage"))
  }

  /// Parse a dimension token
  pub fn dimension() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(
      SimpleToken::Dimension {
        value: 0.0,
        unit: String::new(),
      },
      Some("Dimension"),
    )
  }

  /// Parse a string token
  pub fn string() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::String(String::new()), Some("String"))
  }

  /// Parse a function token
  pub fn function() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::Function(String::new()), Some("Function"))
  }

  /// Parse a hash token
  pub fn hash() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::Hash(String::new()), Some("Hash"))
  }

  /// Parse a delimiter token
  pub fn delim(ch: char) -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::Delim(ch), Some("Delim"))
  }

  /// Parse a whitespace token
  pub fn whitespace() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::Whitespace, Some("Whitespace"))
  }

  /// Parse an at-keyword token
  pub fn at_keyword() -> TokenParser<SimpleToken> {
    TokenParser::<SimpleToken>::token(SimpleToken::AtKeyword(String::new()), Some("AtKeyword"))
  }
}

impl<T: Clone + Debug + 'static> TokenParser<T> {
  pub fn tokens() -> Tokens {
    Tokens
  }

  /// Mixed sequence builder: `mixed_sequence([Left(a), Right(b.optional())]).separated_by(ws)`.
  pub fn mixed_sequence<U: Clone + Debug + 'static>(
    parsers: Vec<Either<TokenParser<U>, TokenParser<Option<U>>>>,
  ) -> MixedSequenceBuilder<U> {
    MixedSequenceBuilder::new(parsers)
  }
}

/// Builder for mixed sequences that can handle optional parsers intelligently
pub struct MixedSequenceBuilder<T: Clone + Debug + 'static> {
  parsers: Vec<Either<TokenParser<T>, TokenParser<Option<T>>>>,
}

impl<T: Clone + Debug + 'static> MixedSequenceBuilder<T> {
  pub fn new(parsers: Vec<Either<TokenParser<T>, TokenParser<Option<T>>>>) -> Self {
    Self { parsers }
  }

  /// Parse with separators, handling optional parsers intelligently
  pub fn separated_by<S: Clone + Debug + 'static>(
    self,
    separator: TokenParser<S>,
  ) -> TokenParser<Vec<Option<T>>> {
    TokenParser::<Vec<Option<T>>>::flexible_sequence_separated_by(self.parsers, separator)
  }
}

/// Consume the next token and check its discriminant matches `expected_token`; on mismatch or
/// end of input, rewind to `saved_index` and return `Err`.
pub fn match_next_token(
  tokens: &mut TokenList,
  saved_index: usize,
  expected_token: &SimpleToken,
) -> Result<SimpleToken, CssParseError> {
  // `consume_next_token` cannot fail; `unwrap_or(None)` folds the impossible `Err` into EOF.
  let result_opt = tokens.consume_next_token().ok().unwrap_or(None);
  match result_opt {
    Some(token) => {
      if std::mem::discriminant(&token) == std::mem::discriminant(expected_token) {
        Ok(token)
      } else {
        tokens.set_current_index(saved_index);
        Err(CssParseError::ParseError {
          message: format!("Expected token type {:?}, got {:?}", expected_token, token),
        })
      }
    },
    None => {
      tokens.set_current_index(saved_index);
      Err(CssParseError::ParseError {
        message: "Expected token, got end of input".to_string(),
      })
    },
  }
}

/// The error a `parse_to_end` run surfaces, or `None` if all input was consumed.
/// Non-generic so its branches monomorphize once.
fn parse_to_end_error(
  tokens: &mut TokenList,
  initial_index: usize,
  label: &str,
  parse_error: Option<String>,
) -> Option<CssParseError> {
  if let Some(error) = parse_error {
    let consumed_tokens = tokens.slice(initial_index, Some(tokens.current_index));
    tokens.set_current_index(initial_index);
    return Some(CssParseError::ParseError {
      message: format!(
        "Expected {} but got {}\nConsumed tokens: {:?}",
        label, error, consumed_tokens
      ),
    });
  }

  // No parse error: ensure all input was consumed.
  if let Some(token) = tokens.peek_infallible() {
    let consumed_tokens = tokens.slice(initial_index, Some(tokens.current_index));
    return Some(CssParseError::ParseError {
      message: format!(
        "Expected end of input, got {:?} instead\nConsumed tokens: {:?}",
        token, consumed_tokens
      ),
    });
  }

  None
}

/// Turn a `collect_set_results` value into the `SetOfParsers` result, rewinding on the
/// (unreachable) `Err`.
pub fn set_of_incomplete_error<T: Clone + Debug>(
  collected: Result<Vec<T>, String>,
  tokens: &mut TokenList,
  start_index: usize,
) -> Result<Vec<T>, CssParseError> {
  match collected {
    Ok(values) => Ok(values),
    Err(err) => {
      tokens.set_current_index(start_index);
      Err(CssParseError::ParseError {
        message: format!("SetOf incomplete: {}", err),
      })
    },
  }
}

/// Collect `Vec<Option<T>>` into `Result`, erroring with the position of any remaining `None`.
pub fn collect_set_results<T>(results: Vec<Option<T>>) -> Result<Vec<T>, String> {
  results
    .into_iter()
    .enumerate()
    .map(|(i, opt)| opt.ok_or_else(|| format!("Parser {} did not match", i)))
    .collect()
}

/// Extract the `String` from a `SimpleToken::Ident`; other variants are unreachable.
pub fn extract_ident_value(token: SimpleToken) -> String {
  if let SimpleToken::Ident(value) = token {
    value
  } else {
    stylex_unreachable!()
  }
}

/// Extract the `String` from a `SimpleToken::Function`; other variants are unreachable.
pub fn extract_function_value(token: SimpleToken) -> String {
  if let SimpleToken::Function(value) = token {
    value
  } else {
    stylex_unreachable!()
  }
}

/// Peek at the next few tokens without consuming them (debugging aid).
pub fn peek_tokens(css: &str, count: usize) -> Vec<SimpleToken> {
  let mut tokens = TokenList::new(css);
  let mut result = Vec::new();
  for _ in 0..count {
    if let Ok(Some(token)) = tokens.peek() {
      result.push(token);
      let _ = tokens.consume_next_token();
    } else {
      break;
    }
  }
  result
}

/// Rewind `tokens` to `saved_index` when `failed`; non-generic so it monomorphizes once.
pub fn rewind_if_err(tokens: &mut TokenList, saved_index: usize, failed: bool) {
  if failed {
    tokens.set_current_index(saved_index);
  }
}

/// Build the label string for `TokenParser::always`; non-generic so it monomorphizes once.
pub fn always_make_label(type_name: &str, debug_str: &str) -> String {
  if type_name == "()" {
    "optional".to_string()
  } else {
    format!("Always<{}>", debug_str)
  }
}

/// Log the outcome of a `TokenParser::debug` call; non-generic so it monomorphizes once.
pub fn debug_log_result(success: bool, label: &str, idx: usize, error: &str) {
  if success {
    debug!(
      "✅ SUCCESS: Parser '{}' matched. Consumed {} tokens.",
      label, idx
    );
  } else {
    debug!(
      "❌ FAILED: Parser '{}' failed at token {}. Error: {}",
      label, idx, error
    );
  }
}

/// Build the error returned by `TokenParser::parse_with_context`; non-generic.
pub fn build_parse_with_context_error(
  error: &CssParseError,
  css: &str,
  fail_idx: usize,
) -> CssParseError {
  let context_tokens = peek_tokens(css, 5);
  let remaining_css = &css[fail_idx.min(css.len())..];
  CssParseError::ParseError {
    message: format!(
      "{}\n📍 Context: Failed at position {} in '{}'\n🔍 Next tokens: {:?}\n📋 Remaining: '{}'",
      error,
      fail_idx,
      css,
      context_tokens,
      remaining_css.chars().take(20).collect::<String>()
    ),
  }
}

/// Build the `TokenParser::one_of` error when all parsers fail; non-generic.
pub fn one_of_error(errors: Vec<CssParseError>) -> CssParseError {
  CssParseError::ParseError {
    message: format!(
      "No parser matched\n{}",
      errors
        .iter()
        .map(|err| format!("- {}", err))
        .collect::<Vec<_>>()
        .join("\n")
    ),
  }
}

#[cfg(test)]
#[path = "tests/token_parser_test.rs"]
mod token_parser_test;

#[cfg(test)]
#[path = "tests/token_parser_coverage_test.rs"]
mod token_parser_coverage_test;
