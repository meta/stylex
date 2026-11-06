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
fn first_returns_none_for_empty_list() {
  let mut list = TokenList::new("");
  let result = list.first().unwrap();
  assert!(
    result.is_none(),
    "first() on empty TokenList should return None"
  );
}

// parse_nested_or_panic: the error arm, unreachable through normal tokenization.

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
