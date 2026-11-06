use super::*;

use cssparser::Token as CssToken;

// map_css_token: variants that never reach it through tokenize_all or tokenize_nested_content.

#[test]
fn map_css_token_function_variant_produces_function_token() {
  // tokenize_all and tokenize_nested_content both handle Function themselves.
  use cssparser::CowRcStr;
  let token = CssToken::Function(CowRcStr::from("rgb"));
  let result = super::map_css_token(&token, "");
  assert_eq!(result, SimpleToken::Function("rgb".to_string()));
}

#[test]
fn map_css_token_delim_open_paren_produces_left_paren() {
  // cssparser emits ParenthesisBlock, never Delim('(').
  let token = CssToken::Delim('(');
  let result = super::map_css_token(&token, "");
  assert_eq!(result, SimpleToken::LeftParen);
}

#[test]
fn map_css_token_delim_close_paren_produces_right_paren() {
  // cssparser emits CloseParenthesis, never Delim(')').
  let token = CssToken::Delim(')');
  let result = super::map_css_token(&token, "");
  assert_eq!(result, SimpleToken::RightParen);
}

// map_css_token via TokenList::new, through the `_ =>` arm of tokenize_all.

#[test]
fn map_css_token_unquoted_url_produces_string_token() {
  let list = TokenList::new("url(foo)");
  assert!(!list.tokens.is_empty());
  let has_string = list
    .tokens
    .iter()
    .any(|t| matches!(t, SimpleToken::String(_)));
  assert!(
    has_string,
    "url(foo) should produce a String token from UnquotedUrl"
  );
}

#[test]
fn map_css_token_bad_url_produces_unknown_token() {
  let list = TokenList::new("url(bad url)");
  assert!(!list.tokens.is_empty());
  let has_unknown = list
    .tokens
    .iter()
    .any(|t| matches!(t, SimpleToken::Unknown(_)));
  assert!(
    has_unknown,
    "url(bad url) should produce an Unknown token from BadUrl"
  );
}

#[test]
fn map_css_token_bad_string_produces_unknown_token() {
  let list = TokenList::new("\"bad\nstring\"");
  assert!(!list.tokens.is_empty());
  let has_unknown = list
    .tokens
    .iter()
    .any(|t| matches!(t, SimpleToken::Unknown(_)));
  assert!(
    has_unknown,
    "newline-inside-string should produce an Unknown token from BadString"
  );
}

#[test]
fn map_css_token_close_parenthesis_produces_right_paren() {
  let list = TokenList::new(")");
  assert!(!list.tokens.is_empty());
  assert_eq!(
    list.tokens[0],
    SimpleToken::RightParen,
    "unmatched ) should produce RightParen"
  );
}

#[test]
fn map_css_token_square_bracket_block_produces_delim_bracket() {
  let list = TokenList::new("[a]");
  assert!(!list.tokens.is_empty());
  assert_eq!(
    list.tokens[0],
    SimpleToken::Delim('['),
    "[ block should produce Delim('[') token"
  );
}

#[test]
fn map_css_token_close_square_bracket_produces_delim_close() {
  let list = TokenList::new("]");
  assert!(!list.tokens.is_empty());
  assert_eq!(
    list.tokens[0],
    SimpleToken::Delim(']'),
    "unmatched ] should produce Delim(']') token"
  );
}

#[test]
fn map_css_token_curly_bracket_block_produces_delim_brace() {
  let list = TokenList::new("{a}");
  assert!(!list.tokens.is_empty());
  assert_eq!(
    list.tokens[0],
    SimpleToken::Delim('{'),
    "{{ block should produce Delim('{{') token"
  );
}

#[test]
fn map_css_token_close_curly_bracket_produces_delim_close_brace() {
  let list = TokenList::new("}");
  assert!(!list.tokens.is_empty());
  assert_eq!(
    list.tokens[0],
    SimpleToken::Delim('}'),
    "unmatched }} should produce Delim('}}') token"
  );
}

#[test]
fn map_css_token_cdc_produces_delim_gt() {
  let list = TokenList::new("-->");
  assert!(!list.tokens.is_empty());
  assert_eq!(
    list.tokens[0],
    SimpleToken::Delim('>'),
    "--> should produce Delim('>') token"
  );
}

#[test]
fn map_css_token_cdo_produces_delim_lt() {
  let list = TokenList::new("<!--");
  assert!(!list.tokens.is_empty());
  assert_eq!(
    list.tokens[0],
    SimpleToken::Delim('<'),
    "<!-- should produce Delim('<') token"
  );
}

#[test]
fn map_css_token_wildcard_produces_unknown_for_comment() {
  let list = TokenList::new("/* hello */");
  assert!(!list.tokens.is_empty());
  let has_unknown = list
    .tokens
    .iter()
    .any(|t| matches!(t, SimpleToken::Unknown(_)));
  assert!(
    has_unknown,
    "CSS comment should produce an Unknown token via the wildcard arm"
  );
}

#[test]
fn map_css_token_wildcard_produces_unknown_for_include_match() {
  let list = TokenList::new("~=");
  assert!(!list.tokens.is_empty());
  let has_unknown = list
    .tokens
    .iter()
    .any(|t| matches!(t, SimpleToken::Unknown(_)));
  assert!(
    has_unknown,
    "~= should produce an Unknown token via the wildcard arm"
  );
}

#[test]
fn nested_paren_inside_function_is_tokenized() {
  // The outer Function enters tokenize_nested_content; the inner `(` hits its ParenthesisBlock arm.
  let list = TokenList::new("rgb((1), 0, 0)");
  assert!(!list.tokens.is_empty());
  let first = &list.tokens[0];
  assert_eq!(*first, SimpleToken::Function("rgb".to_string()));
  let has_left_paren = list.tokens.contains(&SimpleToken::LeftParen);
  assert!(
    has_left_paren,
    "nested ( inside function should produce LeftParen token"
  );
}

#[test]
fn nested_function_inside_function_is_tokenized() {
  // The outer Function enters tokenize_nested_content; the inner `calc(` hits its Function arm.
  let list = TokenList::new("rgb(calc(1px + 2em), 0, 0)");
  assert!(!list.tokens.is_empty());
  assert_eq!(list.tokens[0], SimpleToken::Function("rgb".to_string()));
  let has_calc = list
    .tokens
    .iter()
    .any(|t| *t == SimpleToken::Function("calc".to_string()));
  assert!(
    has_calc,
    "nested calc() inside rgb() should produce a Function token"
  );
}

#[test]
fn nested_content_misc_tokens_mapped_via_map_css_token() {
  // Whitespace inside the function falls to the `_ =>` arm, which calls map_css_token.
  let list = TokenList::new("rgb(255 0 0)");
  let has_whitespace = list.tokens.contains(&SimpleToken::Whitespace);
  assert!(
    has_whitespace,
    "whitespace inside rgb() should be included as Whitespace tokens"
  );
}

#[test]
fn first_returns_same_as_peek_for_non_empty_list() {
  let mut list = TokenList::new("hello");
  let via_peek = list.peek().unwrap();
  let via_first = list.first().unwrap();
  assert_eq!(
    via_peek, via_first,
    "first() should return the same token as peek()"
  );
}

#[test]
fn first_returns_none_for_empty_list() {
  let mut list = TokenList::new("");
  let result = list.first().unwrap();
  assert!(
    result.is_none(),
    "first() on empty TokenList should return None"
  );
}

#[test]
fn first_does_not_consume_token() {
  let mut list = TokenList::new("color");
  let index_before = list.current_index;
  let _ = list.first().unwrap();
  assert_eq!(
    list.current_index, index_before,
    "first() should not advance the current_index"
  );
}

/// `cssparser` 0.38 caps nesting at 75 levels; `tokenize_all` lifts it, as upstream StyleX has
/// no limit.
#[test]
fn a_block_nested_past_the_cssparser_default_limit_is_tokenized() {
  const DEPTH: usize = 100;
  let input = format!("{}a{}", "(".repeat(DEPTH), ")".repeat(DEPTH));

  let tokens = TokenList::new(&input).tokens;

  let count = |wanted: &SimpleToken| tokens.iter().filter(|token| *token == wanted).count();
  assert_eq!(count(&SimpleToken::LeftParen), DEPTH);
  assert_eq!(count(&SimpleToken::RightParen), DEPTH);
  assert_eq!(count(&SimpleToken::Ident("a".to_string())), 1);
}

// parse_nested_or_panic: the error arm, unreachable through normal tokenization.

#[test]
#[should_panic(expected = "Error parsing nested content")]
fn parse_nested_or_panic_panics_when_nested_parse_errors() {
  use cssparser::{ParseError, Parser};

  let mut parser = Parser::new("(a)");
  // Consume the ParenthesisBlock token so a nested parser may be created.
  let _ = parser.next();

  super::parse_nested_or_panic(&mut parser, |_| Err(ParseError::custom(())));
}

/// The guard logs the error before it panics; `catch_unwind` so the log is asserted afterwards.
#[test]
fn a_nested_parse_error_is_written_before_it_stops_the_build() {
  use cssparser::ParseError;
  use log::Level;

  use crate::capturing_logger::logged_at;

  let mut stopped = None;

  let messages = logged_at(Level::Error, || {
    stopped = Some(std::panic::catch_unwind(|| {
      super::handle_nested_block_result(Err(ParseError::custom(())))
    }));
  });

  assert!(
    matches!(stopped, Some(Err(_))),
    "a nested parse error must stop the build"
  );
  assert!(
    messages
      .iter()
      .any(|message| message.starts_with("Error parsing nested content: ")),
    "the error must be written before the build stops, got {messages:?}"
  );
}

/// A nested parse that succeeds logs nothing and does not panic.
#[test]
fn a_nested_block_that_parsed_writes_nothing() {
  use log::Level;

  use crate::capturing_logger::logged_at;

  let messages = logged_at(Level::Error, || super::handle_nested_block_result(Ok(())));

  assert!(messages.is_empty());
}

/// Called directly: no tokenizer input yields a digitless numeric token, and the call sites'
/// `unwrap_or` fallbacks depend on this `None`.
#[test]
fn leading_f64_reads_nothing_from_text_without_digits() {
  assert_eq!(leading_f64(""), None);
  assert_eq!(leading_f64("px"), None);
  assert_eq!(leading_f64("+"), None);
  assert_eq!(leading_f64("-"), None);
  // A lone `.` is a delimiter, not the start of a number.
  assert_eq!(leading_f64("."), None);
  assert_eq!(leading_f64("-.px"), None);
}

/// The prefix stops where the number does; trailing text is ignored.
#[test]
fn leading_f64_reads_the_number_in_front_of_its_unit() {
  assert_eq!(leading_f64("10px"), Some(10.0));
  assert_eq!(leading_f64("-2.5em"), Some(-2.5));
  assert_eq!(leading_f64("+0.75turn"), Some(0.75));
  // A trailing `.` with no digits after it is not part of the number.
  assert_eq!(leading_f64("3.px"), Some(3.0));
  // An exponent only counts when it is complete.
  assert_eq!(leading_f64("1e"), Some(1.0));
  assert_eq!(leading_f64("1e3"), Some(1000.0));
}
