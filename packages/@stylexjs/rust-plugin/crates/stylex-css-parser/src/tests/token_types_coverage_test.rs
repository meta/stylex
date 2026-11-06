use super::*;

// Allow direct access to the private map_css_token function from this child module.
use cssparser::Token as CssToken;

// ---------------------------------------------------------------------------
// map_css_token(): direct calls for token variants that can never reach
// map_css_token through normal tokenize_all / tokenize_nested_content paths.
// Child modules in Rust can access private parent items directly.
// ---------------------------------------------------------------------------

#[test]
fn map_css_token_function_variant_produces_function_token() {
  // CssToken::Function arm — map_css_token is reachable from child
  // modules as a private fn. This variant never reaches map_css_token through
  // tokenize_all/tokenize_nested_content (both handle Function explicitly), so
  // we exercise it by calling map_css_token directly.
  use cssparser::CowRcStr;
  let token = CssToken::Function(CowRcStr::from("rgb"));
  let result = super::map_css_token(&token, "");
  assert_eq!(result, SimpleToken::Function("rgb".to_string()));
}

#[test]
fn map_css_token_delim_open_paren_produces_left_paren() {
  // CssToken::Delim('(') arm — cssparser never emits this variant
  // at the token level (it emits ParenthesisBlock instead), so we call
  // map_css_token directly.
  let token = CssToken::Delim('(');
  let result = super::map_css_token(&token, "");
  assert_eq!(result, SimpleToken::LeftParen);
}

#[test]
fn map_css_token_delim_close_paren_produces_right_paren() {
  // CssToken::Delim(')') arm — cssparser emits CloseParenthesis
  // instead; we call map_css_token directly.
  let token = CssToken::Delim(')');
  let result = super::map_css_token(&token, "");
  assert_eq!(result, SimpleToken::RightParen);
}

// ---------------------------------------------------------------------------
// map_css_token arms via TokenList::new (top-level tokens fall through to the
// `_ =>` arm in tokenize_all, which calls map_css_token).
// ---------------------------------------------------------------------------

#[test]
fn map_css_token_unquoted_url_produces_string_token() {
  // url(foo) => CssToken::UnquotedUrl => SimpleToken::String
  let list = TokenList::new("url(foo)");
  // The tokenizer sees url( and produces UnquotedUrl for bare-word urls
  assert!(!list.tokens.is_empty());
  // Verify at least one token is a String (from UnquotedUrl)
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
  // url(bad url) => CssToken::BadUrl => SimpleToken::Unknown
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
  // "bad\nstring" => CssToken::BadString => SimpleToken::Unknown
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
  // Unmatched ) at top level => CssToken::CloseParenthesis => SimpleToken::RightParen
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
  // [a] => CssToken::SquareBracketBlock => SimpleToken::Delim('[')
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
  // Unmatched ] at top level => CssToken::CloseSquareBracket => SimpleToken::Delim(']')
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
  // {a} => CssToken::CurlyBracketBlock => SimpleToken::Delim('{')
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
  // Unmatched } at top level => CssToken::CloseCurlyBracket => SimpleToken::Delim('}')
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
  // --> (CSS comment-close) => CssToken::CDC => SimpleToken::Delim('>')
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
  // <!-- (HTML comment-open) => CssToken::CDO => SimpleToken::Delim('<')
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
  // /* comment */ => CssToken::Comment (no explicit arm) => SimpleToken::Unknown
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
  // ~= => CssToken::IncludeMatch (no explicit arm) => SimpleToken::Unknown
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

// ---------------------------------------------------------------------------
// tokenize_nested_content: nested ParenthesisBlock inside a function
// ---------------------------------------------------------------------------

#[test]
fn nested_paren_inside_function_is_tokenized() {
  // rgb((1), 0, 0) — the outer Function token triggers tokenize_nested_content,
  // then the inner ( triggers the ParenthesisBlock arm.
  let list = TokenList::new("rgb((1), 0, 0)");
  assert!(!list.tokens.is_empty());
  // Expect tokens: Function("rgb"), LeftParen, Number(1.0), RightParen, ...
  let first = &list.tokens[0];
  assert_eq!(*first, SimpleToken::Function("rgb".to_string()));
  // The nested ( inside the function body must also appear
  let has_left_paren = list.tokens.contains(&SimpleToken::LeftParen);
  assert!(
    has_left_paren,
    "nested ( inside function should produce LeftParen token"
  );
}

// ---------------------------------------------------------------------------
// tokenize_nested_content: nested Function inside a function
// ---------------------------------------------------------------------------

#[test]
fn nested_function_inside_function_is_tokenized() {
  // rgb(calc(1px + 2em), 0, 0) — the outer Function triggers tokenize_nested_content,
  // then the inner calc( triggers the Function arm.
  let list = TokenList::new("rgb(calc(1px + 2em), 0, 0)");
  assert!(!list.tokens.is_empty());
  assert_eq!(list.tokens[0], SimpleToken::Function("rgb".to_string()));
  // calc should appear as a nested Function token
  let has_calc = list
    .tokens
    .iter()
    .any(|t| *t == SimpleToken::Function("calc".to_string()));
  assert!(
    has_calc,
    "nested calc() inside rgb() should produce a Function token"
  );
}

// ---------------------------------------------------------------------------
// tokenize_nested_content: `_ =>` arm called with misc tokens
// This is already exercised by rgb(1, 2, 3) but we add an explicit assertion
// for the Whitespace and Comma tokens that appear in the nested block.
// ---------------------------------------------------------------------------

#[test]
fn nested_content_misc_tokens_mapped_via_map_css_token() {
  // Inside rgb(255 0 0), the whitespace tokens fall to the `_ =>` arm which
  // calls map_css_token.
  let list = TokenList::new("rgb(255 0 0)");
  let has_whitespace = list.tokens.contains(&SimpleToken::Whitespace);
  assert!(
    has_whitespace,
    "whitespace inside rgb() should be included as Whitespace tokens"
  );
}

// ---------------------------------------------------------------------------
// TokenList::first() — alias for peek()
// ---------------------------------------------------------------------------

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

/// `cssparser` 0.38 refuses a block nested deeper than 75 levels by default.
/// `tokenize_all` removes that limit, because upstream StyleX has no limit.
/// If the limit comes back, the nested parse fails and the guard stops the
/// build. This test makes sure that the tokenizer reads every level.
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

// ---------------------------------------------------------------------------
// parse_nested_or_panic(): the error arm.
//
// In normal tokenization the closure always returns Ok, so the panic guard is
// never hit. We exercise it directly: position a real cssparser `Parser` just
// after a block-opening token, then hand `parse_nested_or_panic` a closure that
// returns `Err`. `parse_nested_block` surfaces that error and the guard panics.
// ---------------------------------------------------------------------------

#[test]
#[should_panic(expected = "Error parsing nested content")]
fn parse_nested_or_panic_panics_when_nested_parse_errors() {
  use cssparser::{ParseError, Parser};

  let mut parser = Parser::new("(a)");
  // Consume the ParenthesisBlock token so a nested parser may be created.
  let _ = parser.next();

  super::parse_nested_or_panic(&mut parser, |_| Err(ParseError::custom(())));
}

/// The guard writes the error before it stops the build, and that message is
/// the only account of what went wrong. It is built from the error, which a
/// `log` macro does not build while no logger admits the level.
///
/// `catch_unwind` rather than `should_panic`: the message has to be asserted
/// after the panic, which a `should_panic` case never reaches.
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

/// The same guard on the answer it is given almost every time. Nothing is
/// written and nothing stops.
#[test]
fn a_nested_block_that_parsed_writes_nothing() {
  use log::Level;

  use crate::capturing_logger::logged_at;

  let messages = logged_at(Level::Error, || super::handle_nested_block_result(Ok(())));

  assert!(messages.is_empty());
}

// ---------------------------------------------------------------------------
// leading_f64(): the paths where there is no number to read
// ---------------------------------------------------------------------------

/// `leading_f64` reads the numeric prefix of a token's raw text. Every caller
/// hands it the text of a token `cssparser` already classified as numeric, so
/// through the parser there is always a number there -- but the function is
/// written to answer honestly when there is not, and that answer is what the
/// `unwrap_or` fallbacks at each call site depend on.
///
/// Called directly because no tokenizer input can produce a `Number` token
/// whose text has no digits; a test routed through the parser would assert the
/// tokenizer's classification rather than this function's contract.
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

/// The prefix stops where the number does, so trailing text is ignored rather
/// than making the whole read fail.
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
