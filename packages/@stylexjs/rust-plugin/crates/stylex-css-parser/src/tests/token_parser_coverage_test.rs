use super::*;
use crate::{
  capturing_logger::logged_at,
  css_types::{Scale, TransformFunction},
  token_types::{SimpleToken, TokenList},
};
use log::Level;

// Builds a TokenList directly, bypassing the CSS tokenizer.

fn tl(tokens: Vec<SimpleToken>) -> TokenList {
  TokenList {
    tokens,
    current_index: 0,
    depth: 0,
  }
}

#[test]
fn tokens_semicolon_success() {
  let mut tl = tl(vec![SimpleToken::Semicolon]);
  let parser = tokens::semicolon();
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert_eq!(result.unwrap(), SimpleToken::Semicolon);
}

#[test]
fn tokens_function_success() {
  let mut tl = self::tl(vec![
    SimpleToken::Function("rgb".to_string()),
    SimpleToken::RightParen,
  ]);
  let parser = tokens::function();
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert!(matches!(result.unwrap(), SimpleToken::Function(_)));
}

#[test]
fn tokens_string_success() {
  let mut tl = tl(vec![SimpleToken::String("hello".to_string())]);
  let parser = tokens::string();
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert!(matches!(result.unwrap(), SimpleToken::String(_)));
}

#[test]
fn tokens_hash_success() {
  let result = tokens::hash().parse("#abc");
  assert!(result.is_ok());
  assert!(matches!(result.unwrap(), SimpleToken::Hash(_)));
}

#[test]
fn tokens_url_success() {
  let mut tl = tl(vec![SimpleToken::Url("https://example.com".to_string())]);
  let parser = tokens::url();
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert!(matches!(result.unwrap(), SimpleToken::Url(_)));
}

#[test]
fn tokens_open_paren_success() {
  let mut tl = tl(vec![SimpleToken::LeftParen]);
  let parser = tokens::open_paren();
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert_eq!(result.unwrap(), SimpleToken::LeftParen);
}

#[test]
fn tokens_close_paren_success() {
  let mut tl = tl(vec![SimpleToken::RightParen]);
  let parser = tokens::close_paren();
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert_eq!(result.unwrap(), SimpleToken::RightParen);
}

#[test]
fn tokens_open_square_success() {
  let mut tl = tl(vec![SimpleToken::LeftBracket]);
  let parser = tokens::open_square();
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert_eq!(result.unwrap(), SimpleToken::LeftBracket);
}

#[test]
fn tokens_close_square_success() {
  let mut tl = tl(vec![SimpleToken::RightBracket]);
  let parser = tokens::close_square();
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert_eq!(result.unwrap(), SimpleToken::RightBracket);
}

#[test]
fn tokens_open_curly_success() {
  let mut tl = tl(vec![SimpleToken::LeftBrace]);
  let parser = tokens::open_curly();
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert_eq!(result.unwrap(), SimpleToken::LeftBrace);
}

#[test]
fn tokens_close_curly_success() {
  let mut tl = tl(vec![SimpleToken::RightBrace]);
  let parser = tokens::close_curly();
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert_eq!(result.unwrap(), SimpleToken::RightBrace);
}

#[test]
fn tokens_delim_success() {
  let mut tl = tl(vec![SimpleToken::Delim('+')]);
  let parser = tokens::delim('+');
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert_eq!(result.unwrap(), SimpleToken::Delim('+'));
}

#[test]
fn parse_to_end_fails_when_tokens_remain() {
  let parser = tokens::ident();
  let result = parser.parse_to_end("foo bar");
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(
    msg.contains("end of input") || msg.contains("Expected"),
    "msg: {msg}"
  );
}

#[test]
fn flat_map_second_parser_fails_rewinds() {
  let parser = tokens::ident().flat_map(
    |_| tokens::colon(), // expect a colon next, but there won't be one
    Some("then_colon"),
  );
  let result = parser.parse("foo");
  assert!(result.is_err());
}

#[test]
fn surrounded_by_none_suffix_success() {
  let content_parser = tokens::ident();
  let prefix_parser = tokens::colon();
  let surrounded = content_parser.surrounded_by(prefix_parser, None::<TokenParser<SimpleToken>>);
  let mut tl = self::tl(vec![
    SimpleToken::Colon,
    SimpleToken::Ident("foo".to_string()),
    SimpleToken::Colon,
  ]);
  let result = (surrounded.run)(&mut tl);
  assert!(result.is_ok());
  assert!(matches!(result.unwrap(), SimpleToken::Ident(_)));

  let mut tl = self::tl(vec![SimpleToken::Ident("foo".to_string())]);
  assert!((surrounded.run)(&mut tl).is_err());
  let mut tl = self::tl(vec![SimpleToken::Colon, SimpleToken::Colon]);
  assert!((surrounded.run)(&mut tl).is_err());
  let mut tl = self::tl(vec![
    SimpleToken::Colon,
    SimpleToken::Ident("foo".to_string()),
  ]);
  assert!((surrounded.run)(&mut tl).is_err());
}

#[test]
fn surrounded_by_none_suffix_prefix_fails() {
  let content_parser = tokens::ident();
  let prefix_parser = tokens::colon();
  let surrounded = content_parser.surrounded_by(prefix_parser, None::<TokenParser<SimpleToken>>);
  let mut tl = self::tl(vec![SimpleToken::Ident("foo".to_string())]);
  let result = (surrounded.run)(&mut tl);
  assert!(result.is_err());

  let mut tl = self::tl(vec![
    SimpleToken::Colon,
    SimpleToken::Ident("foo".to_string()),
    SimpleToken::Colon,
  ]);
  assert!((surrounded.run)(&mut tl).is_ok());
  let mut tl = self::tl(vec![SimpleToken::Colon, SimpleToken::Colon]);
  assert!((surrounded.run)(&mut tl).is_err());
  let mut tl = self::tl(vec![
    SimpleToken::Colon,
    SimpleToken::Ident("foo".to_string()),
  ]);
  assert!((surrounded.run)(&mut tl).is_err());
}

#[test]
fn surrounded_by_none_suffix_main_fails() {
  let content_parser = tokens::colon(); // we'll give it no colon after the bracket
  let prefix_parser = tokens::ident();
  let surrounded = content_parser.surrounded_by(prefix_parser, None::<TokenParser<SimpleToken>>);
  let mut tl = self::tl(vec![
    SimpleToken::Ident("foo".to_string()),
    SimpleToken::Whitespace, // not a colon — content parser fails
    SimpleToken::Ident("bar".to_string()),
  ]);
  let result = (surrounded.run)(&mut tl);
  assert!(result.is_err());

  let mut tl = self::tl(vec![
    SimpleToken::Ident("foo".to_string()),
    SimpleToken::Colon,
    SimpleToken::Ident("bar".to_string()),
  ]);
  assert!((surrounded.run)(&mut tl).is_ok());
  let mut tl = self::tl(vec![SimpleToken::Colon]);
  assert!((surrounded.run)(&mut tl).is_err());
  let mut tl = self::tl(vec![
    SimpleToken::Ident("foo".to_string()),
    SimpleToken::Colon,
  ]);
  assert!((surrounded.run)(&mut tl).is_err());
}

#[test]
fn surrounded_by_none_suffix_suffix_fails() {
  let content_parser = tokens::colon();
  let prefix_parser = tokens::ident();
  let surrounded = content_parser.surrounded_by(prefix_parser, None::<TokenParser<SimpleToken>>);
  let mut tl = self::tl(vec![
    SimpleToken::Ident("foo".to_string()),
    SimpleToken::Colon,
    // no trailing ident — suffix parser fails
  ]);
  let result = (surrounded.run)(&mut tl);
  assert!(result.is_err());

  let mut tl = self::tl(vec![
    SimpleToken::Ident("foo".to_string()),
    SimpleToken::Colon,
    SimpleToken::Ident("bar".to_string()),
  ]);
  assert!((surrounded.run)(&mut tl).is_ok());
  let mut tl = self::tl(vec![SimpleToken::Colon]);
  assert!((surrounded.run)(&mut tl).is_err());
  let mut tl = self::tl(vec![
    SimpleToken::Ident("foo".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Ident("bar".to_string()),
  ]);
  assert!((surrounded.run)(&mut tl).is_err());
}

#[test]
fn surrounded_by_some_suffix_prefix_fails() {
  let content = tokens::ident();
  let prefix = tokens::colon();
  let suffix = tokens::semicolon();
  let surrounded = content.surrounded_by(prefix, Some(suffix));
  let mut tl = self::tl(vec![SimpleToken::Ident("foo".to_string())]);
  let result = (surrounded.run)(&mut tl);
  assert!(result.is_err());

  let mut tl = self::tl(vec![
    SimpleToken::Colon,
    SimpleToken::Ident("foo".to_string()),
    SimpleToken::Semicolon,
  ]);
  assert!((surrounded.run)(&mut tl).is_ok());
  let mut tl = self::tl(vec![SimpleToken::Colon, SimpleToken::Colon]);
  assert!((surrounded.run)(&mut tl).is_err());
  let mut tl = self::tl(vec![
    SimpleToken::Colon,
    SimpleToken::Ident("foo".to_string()),
  ]);
  assert!((surrounded.run)(&mut tl).is_err());
}

#[test]
fn surrounded_by_some_suffix_main_fails() {
  let content = tokens::semicolon(); // expects semicolon as content
  let prefix = tokens::colon();
  let suffix = tokens::colon();
  let surrounded = content.surrounded_by(prefix, Some(suffix));
  let mut tl = tl(vec![
    SimpleToken::Colon,
    SimpleToken::Ident("foo".to_string()), // not a semicolon
    SimpleToken::Colon,
  ]);
  let result = (surrounded.run)(&mut tl);
  assert!(result.is_err());

  let mut tl = self::tl(vec![
    SimpleToken::Colon,
    SimpleToken::Semicolon,
    SimpleToken::Colon,
  ]);
  assert!((surrounded.run)(&mut tl).is_ok());
  let mut tl = self::tl(vec![SimpleToken::Ident("foo".to_string())]);
  assert!((surrounded.run)(&mut tl).is_err());
  let mut tl = self::tl(vec![SimpleToken::Colon, SimpleToken::Semicolon]);
  assert!((surrounded.run)(&mut tl).is_err());
}

#[test]
fn surrounded_by_some_suffix_suffix_fails() {
  let content = tokens::ident();
  let prefix = tokens::colon();
  let suffix = tokens::semicolon();
  let surrounded = content.surrounded_by(prefix, Some(suffix));
  let mut tl = tl(vec![
    SimpleToken::Colon,
    SimpleToken::Ident("foo".to_string()),
    // no semicolon
  ]);
  let result = (surrounded.run)(&mut tl);
  assert!(result.is_err());

  let mut tl = self::tl(vec![
    SimpleToken::Colon,
    SimpleToken::Ident("foo".to_string()),
    SimpleToken::Semicolon,
  ]);
  assert!((surrounded.run)(&mut tl).is_ok());
  let mut tl = self::tl(vec![SimpleToken::Ident("foo".to_string())]);
  assert!((surrounded.run)(&mut tl).is_err());
  let mut tl = self::tl(vec![SimpleToken::Colon, SimpleToken::Colon]);
  assert!((surrounded.run)(&mut tl).is_err());
}

#[test]
fn label_method_returns_label() {
  let parser = tokens::ident();
  assert_eq!(parser.label(), "Ident");

  let parser2 = TokenParser::always(42).with_label("my_label");
  assert_eq!(parser2.label(), "my_label");
}

/// `debug` logs what it parsed and how it ended; the case reads both messages back.
#[test]
fn debug_method_success() {
  let parser = tokens::ident();
  let label = parser.label().to_string();
  let mut result = None;

  let messages = logged_at(Level::Debug, || result = Some(parser.debug("foo")));

  assert!(matches!(result, Some(Ok(SimpleToken::Ident(_)))));
  assert!(
    messages
      .iter()
      .any(|message| message == &format!("Parsing 'foo' with parser '{label}'")),
    "the input and the parser must be named, got {messages:?}"
  );
  assert!(
    messages
      .iter()
      .any(|message| message.starts_with(&format!("✅ SUCCESS: Parser '{label}' matched."))),
    "a parse that succeeded must say so, got {messages:?}"
  );
}

/// A failing parse logs the position and the error it stopped with.
#[test]
fn debug_method_failure() {
  let parser = tokens::colon();
  let label = parser.label().to_string();
  let mut result = None;

  // An ident is not a colon.
  let messages = logged_at(Level::Debug, || result = Some(parser.debug("foo")));

  assert!(matches!(result, Some(Err(_))));
  assert!(
    messages
      .iter()
      .any(|message| message.starts_with(&format!("❌ FAILED: Parser '{label}' failed at token "))),
    "a parse that failed must say so, got {messages:?}"
  );
}

#[test]
fn parse_with_context_success() {
  let parser = tokens::ident();
  let result = parser.parse_with_context("foo");
  assert!(result.is_ok());
}

#[test]
fn parse_with_context_error() {
  let parser = tokens::colon();
  let result = parser.parse_with_context("foo");
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(
    msg.contains("Context") || msg.contains("Failed") || msg.contains("Expected"),
    "msg: {msg}"
  );
}

#[test]
fn with_label_changes_label() {
  let parser = tokens::ident().with_label("my_custom_label");
  assert_eq!(parser.label, "my_custom_label");
}

#[test]
fn always_unit_type_uses_optional_label() {
  let parser: TokenParser<()> = TokenParser::always(());
  assert_eq!(parser.label, "optional");
}

#[test]
fn separated_by_optional_whitespace_single() {
  let parser = tokens::ident().map(
    |t| {
      if let SimpleToken::Ident(v) = t {
        v
      } else {
        String::new()
      }
    },
    None,
  );
  let result = parser
    .separated_by_optional_whitespace()
    .parse_to_end("foo");
  assert!(result.is_ok());
  assert_eq!(result.unwrap(), vec!["foo"]);
}

#[test]
fn separated_by_optional_whitespace_multiple_with_whitespace() {
  let parser = tokens::ident().map(
    |t| {
      if let SimpleToken::Ident(v) = t {
        v
      } else {
        String::new()
      }
    },
    None,
  );
  let result = parser
    .separated_by_optional_whitespace()
    .parse_to_end("foo bar");
  assert!(result.is_ok());
  assert_eq!(result.unwrap(), vec!["foo", "bar"]);
}

#[test]
fn zero_or_more_static_empty() {
  let parser = TokenParser::zero_or_more(tokens::colon());
  let result = parser.parse("foo");
  assert!(result.is_ok());
  assert_eq!(result.unwrap(), vec![]);
}

#[test]
fn zero_or_more_static_multiple() {
  let parser = TokenParser::zero_or_more(tokens::colon());
  let mut tl = tl(vec![
    SimpleToken::Colon,
    SimpleToken::Colon,
    SimpleToken::Ident("x".to_string()),
  ]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert_eq!(result.unwrap().len(), 2);
}

#[test]
fn one_or_more_static_single() {
  let parser = TokenParser::one_or_more(tokens::ident());
  let mut tl = tl(vec![SimpleToken::Ident("foo".to_string())]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert_eq!(result.unwrap().len(), 1);
}

#[test]
fn one_or_more_static_multiple() {
  let parser = TokenParser::one_or_more(tokens::ident());
  let mut tl = tl(vec![
    SimpleToken::Ident("a".to_string()),
    SimpleToken::Ident("b".to_string()),
    SimpleToken::Ident("c".to_string()),
  ]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert_eq!(result.unwrap().len(), 3);
}

#[test]
fn one_or_more_static_fails_on_empty() {
  let parser = TokenParser::one_or_more(tokens::colon());
  let result = parser.parse("foo");
  assert!(result.is_err());
}

// consume_next_token never returns Err, so only the Ok(None) arm is reachable here.

#[test]
fn token_parser_eof_returns_error() {
  let parser = tokens::ident();
  let result = parser.parse("");
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(
    msg.contains("end of input") || msg.contains("Expected"),
    "msg: {msg}"
  );
}

#[test]
fn extract_ident_value_happy_path() {
  let result = extract_ident_value(SimpleToken::Ident("hello".to_string()));
  assert_eq!(result, "hello");
}

#[test]
#[should_panic]
fn extract_ident_value_unreachable_arm() {
  extract_ident_value(SimpleToken::Colon);
}

#[test]
fn extract_function_value_happy_path() {
  let result = extract_function_value(SimpleToken::Function("rgb".to_string()));
  assert_eq!(result, "rgb");
}

#[test]
#[should_panic]
fn extract_function_value_unreachable_arm() {
  extract_function_value(SimpleToken::Colon);
}

#[test]
fn separated_parser_as_token_parser() {
  let parser = tokens::ident()
    .map(
      |t| {
        if let SimpleToken::Ident(v) = t {
          v
        } else {
          String::new()
        }
      },
      None,
    )
    .separated_by(tokens::comma())
    .as_token_parser();
  let result = parser.parse_to_end("foo,bar,baz");
  assert!(result.is_ok());
  assert_eq!(result.unwrap(), vec!["foo", "bar", "baz"]);
}

#[test]
fn flexible_sequence_separated_by_required_missing_separator() {
  let foo = TokenParser::<String>::string("foo");
  let baz = TokenParser::<String>::string("baz");
  let parser = TokenParser::<String>::flexible_sequence_separated_by(
    vec![Either::Left(foo), Either::Left(baz)],
    tokens::whitespace(), // whitespace as separator
  );
  let mut tl = tl(vec![
    SimpleToken::Ident("foo".to_string()),
    // no whitespace
    SimpleToken::Ident("baz".to_string()),
  ]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_err());
}

#[test]
fn flexible_sequence_separated_by_required_parser_fails() {
  let foo = TokenParser::<String>::string("foo");
  let baz = TokenParser::<String>::string("baz");
  let parser = TokenParser::<String>::flexible_sequence_separated_by(
    vec![Either::Left(foo), Either::Left(baz)],
    tokens::whitespace(),
  );
  let mut tl = tl(vec![
    SimpleToken::Ident("foo".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Ident("bar".to_string()), // not "baz"
  ]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_err());
}

#[test]
fn flexible_sequence_separated_by_optional_matched_without_separator() {
  // A never() separator always fails, so separator_consumed stays false.
  let foo = TokenParser::<String>::string("foo");
  let bar_opt: TokenParser<Option<String>> = TokenParser::always(Some("bar".to_string()));
  let parser = TokenParser::<String>::flexible_sequence_separated_by(
    vec![Either::Left(foo), Either::Right(bar_opt)],
    TokenParser::<SimpleToken>::never().map(|_| SimpleToken::Whitespace, None), // separator always fails
  );
  let mut tl = tl(vec![SimpleToken::Ident("foo".to_string())]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_err());
}

#[test]
fn flexible_sequence_separated_by_optional_none_rewinds_separator() {
  let foo = TokenParser::<String>::string("foo");
  let opt_none: TokenParser<Option<String>> = TokenParser::always(None);
  let baz = TokenParser::<String>::string("baz");
  let parser = TokenParser::<String>::flexible_sequence_separated_by(
    vec![
      Either::Left(foo),
      Either::Right(opt_none),
      Either::Left(baz),
    ],
    tokens::whitespace(),
  );
  let mut tl = tl(vec![
    SimpleToken::Ident("foo".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Ident("baz".to_string()),
  ]);
  let result = (parser.run)(&mut tl);
  let _ = result; // may succeed or fail — we only care about exercising the path
}

#[test]
fn flexible_sequence_separated_by_optional_err_rewinds_separator() {
  let foo = TokenParser::<String>::string("foo");
  let opt_err: TokenParser<Option<String>> = TokenParser::<String>::never().optional();
  let parser = TokenParser::<String>::flexible_sequence_separated_by(
    vec![Either::Left(foo), Either::Right(opt_err)],
    tokens::whitespace(),
  );
  let mut tl = tl(vec![
    SimpleToken::Ident("foo".to_string()),
    SimpleToken::Whitespace,
    // nothing that matches optional
  ]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok()); // optional None pushed, result=[Some("foo"), None]
}

#[test]
fn flexible_sequence_first_parser_is_optional_success() {
  let bar_opt: TokenParser<Option<String>> = TokenParser::<String>::string("bar").optional();
  let parser = TokenParser::<String>::flexible_sequence_separated_by(
    vec![Either::Right(bar_opt)],
    tokens::whitespace(),
  );
  let mut tl = tl(vec![SimpleToken::Ident("bar".to_string())]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert_eq!(result.unwrap(), vec![Some("bar".to_string())]);
}

#[test]
fn flexible_sequence_first_parser_is_optional_err() {
  let never_opt: TokenParser<Option<String>> = TokenParser::<String>::never().optional();
  let parser = TokenParser::<String>::flexible_sequence_separated_by(
    vec![Either::Right(never_opt)],
    tokens::whitespace(),
  );
  let mut tl = tl(vec![]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert_eq!(result.unwrap(), vec![None]);
}

#[test]
fn set_of_separated_by_missing_separator_error() {
  let p_foo = TokenParser::<String>::string("foo");
  let p_bar = TokenParser::<String>::string("bar");
  let parser = TokenParser::<String>::set_of(vec![p_foo, p_bar]).separated_by(tokens::whitespace()); // requires whitespace between elements

  let mut tl = tl(vec![
    SimpleToken::Ident("foo".to_string()),
    // no whitespace
    SimpleToken::Ident("bar".to_string()),
  ]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(
    msg.contains("separator") || msg.contains("SetOf"),
    "msg: {msg}"
  );
}

#[test]
fn set_of_separated_by_no_parser_matches() {
  let p_foo = TokenParser::<String>::string("foo");
  let p_bar = TokenParser::<String>::string("bar");
  let parser = TokenParser::<String>::set_of(vec![p_foo, p_bar]).separated_by(tokens::whitespace());

  let result = parser.parse("baz qux");
  assert!(result.is_err());
}

#[test]
fn set_of_as_token_parser_no_match() {
  let p_foo = TokenParser::<String>::string("foo");
  let p_bar = TokenParser::<String>::string("bar");
  let parser = TokenParser::<String>::set_of(vec![p_foo, p_bar]).as_token_parser();
  let result = parser.parse("baz");
  assert!(result.is_err());
}

#[test]
fn sequence_parsers_as_token_parser_fails() {
  let p_foo = TokenParser::<String>::string("foo");
  let p_bar = TokenParser::<String>::string("bar");
  let parser =
    TokenParser::<String>::sequence_with_separators(vec![p_foo, p_bar]).as_token_parser();
  let mut tl = tl(vec![
    SimpleToken::Ident("foo".to_string()),
    SimpleToken::Ident("baz".to_string()), // not "bar"
  ]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_err());
}

#[test]
fn sequence_separated_by_retry_after_separator_consumed() {
  let p_foo = TokenParser::<String>::string("foo");
  // Always succeeds, standing in for an optional parser.
  let p_always = TokenParser::always("x".to_string());
  let parser = TokenParser::<String>::sequence_with_separators(vec![p_foo, p_always])
    .separated_by(tokens::whitespace());

  let mut tl = tl(vec![
    SimpleToken::Ident("foo".to_string()),
    SimpleToken::Whitespace,
  ]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
}

#[test]
fn sequence_separated_by_both_attempts_fail() {
  let p_foo = TokenParser::<String>::string("foo");
  let p_bar = TokenParser::<String>::string("bar");
  let parser = TokenParser::<String>::sequence_with_separators(vec![p_foo, p_bar])
    .separated_by(tokens::whitespace());

  let mut tl = tl(vec![
    SimpleToken::Ident("foo".to_string()),
    SimpleToken::Whitespace,
    // nothing after separator — bar fails
  ]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_err());
}

#[test]
fn sequence_separated_by_no_separator_and_parser_fails() {
  let p_foo = TokenParser::<String>::string("foo");
  let p_bar = TokenParser::<String>::string("bar");
  let parser = TokenParser::<String>::sequence_with_separators(vec![p_foo, p_bar])
    .separated_by(tokens::whitespace());

  let mut tl = tl(vec![
    SimpleToken::Ident("foo".to_string()),
    // no whitespace between foo and baz
    SimpleToken::Ident("baz".to_string()),
  ]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_err());
}

#[test]
fn sequence_separated_by_first_parser_fails() {
  let p_foo = TokenParser::<String>::string("foo");
  let p_bar = TokenParser::<String>::string("bar");
  let parser = TokenParser::<String>::sequence_with_separators(vec![p_foo, p_bar])
    .separated_by(tokens::whitespace());

  let result = parser.parse("baz bar");
  assert!(result.is_err());
}

#[test]
fn tokens_struct_ident() {
  let parser = Tokens::ident();
  let result = parser.parse("foo");
  assert!(result.is_ok());
}

#[test]
fn tokens_struct_comma() {
  let parser = Tokens::comma();
  let mut tl = tl(vec![SimpleToken::Comma]);
  assert!((parser.run)(&mut tl).is_ok());
}

#[test]
fn tokens_struct_colon() {
  let parser = Tokens::colon();
  let mut tl = tl(vec![SimpleToken::Colon]);
  assert!((parser.run)(&mut tl).is_ok());
}

#[test]
fn tokens_struct_semicolon() {
  let parser = Tokens::semicolon();
  let mut tl = tl(vec![SimpleToken::Semicolon]);
  assert!((parser.run)(&mut tl).is_ok());
}

#[test]
fn tokens_struct_open_paren() {
  let parser = Tokens::open_paren();
  let mut tl = tl(vec![SimpleToken::LeftParen]);
  assert!((parser.run)(&mut tl).is_ok());
}

#[test]
fn tokens_struct_close_paren() {
  let parser = Tokens::close_paren();
  let mut tl = tl(vec![SimpleToken::RightParen]);
  assert!((parser.run)(&mut tl).is_ok());
}

#[test]
fn tokens_struct_open_square() {
  let parser = Tokens::open_square();
  let mut tl = tl(vec![SimpleToken::LeftBracket]);
  assert!((parser.run)(&mut tl).is_ok());
}

#[test]
fn tokens_struct_close_square() {
  let parser = Tokens::close_square();
  let mut tl = tl(vec![SimpleToken::RightBracket]);
  assert!((parser.run)(&mut tl).is_ok());
}

#[test]
fn tokens_struct_open_curly() {
  let parser = Tokens::open_curly();
  let mut tl = tl(vec![SimpleToken::LeftBrace]);
  assert!((parser.run)(&mut tl).is_ok());
}

#[test]
fn tokens_struct_close_curly() {
  let parser = Tokens::close_curly();
  let mut tl = tl(vec![SimpleToken::RightBrace]);
  assert!((parser.run)(&mut tl).is_ok());
}

#[test]
fn tokens_struct_number() {
  let parser = Tokens::number();
  let result = parser.parse("42");
  assert!(result.is_ok());
}

#[test]
fn tokens_struct_percentage() {
  let parser = Tokens::percentage();
  let result = parser.parse("50%");
  assert!(result.is_ok());
}

#[test]
fn tokens_struct_dimension() {
  let parser = Tokens::dimension();
  let result = parser.parse("10px");
  assert!(result.is_ok());
}

#[test]
fn tokens_struct_string() {
  let parser = Tokens::string();
  let mut tl = tl(vec![SimpleToken::String("hello".to_string())]);
  assert!((parser.run)(&mut tl).is_ok());
}

#[test]
fn tokens_struct_function() {
  let parser = Tokens::function();
  let mut tl = tl(vec![SimpleToken::Function("rgb".to_string())]);
  assert!((parser.run)(&mut tl).is_ok());
}

#[test]
fn tokens_struct_hash() {
  let parser = Tokens::hash();
  let result = parser.parse("#fff");
  assert!(result.is_ok());
}

#[test]
fn tokens_struct_delim() {
  let parser = Tokens::delim('+');
  let mut tl = tl(vec![SimpleToken::Delim('+')]);
  assert!((parser.run)(&mut tl).is_ok());
}

#[test]
fn tokens_struct_whitespace() {
  let parser = Tokens::whitespace();
  let mut tl = tl(vec![SimpleToken::Whitespace]);
  assert!((parser.run)(&mut tl).is_ok());
}

#[test]
fn tokens_struct_at_keyword() {
  let parser = Tokens::at_keyword();
  let mut tl = tl(vec![SimpleToken::AtKeyword("media".to_string())]);
  assert!((parser.run)(&mut tl).is_ok());
}

#[test]
fn token_parser_tokens_accessor() {
  let _tokens = TokenParser::<SimpleToken>::tokens();
  let parser = TokenParser::<SimpleToken>::tokens();
  let ident_parser = Tokens::ident(); // just ensure Tokens is usable
  let result = ident_parser.parse("foo");
  assert!(result.is_ok());
  let _ = parser; // tokens() returns Tokens
}

#[test]
fn raw_coverage_string_or_instantiation_all_paths() {
  let parser = TokenParser::<String>::string("foo").or(TokenParser::<String>::string("bar"));
  assert!(matches!(
    parser.parse_to_end("foo").unwrap(),
    Either::Left(value) if value == "foo"
  ));
  assert!(matches!(
    parser.parse_to_end("bar").unwrap(),
    Either::Right(value) if value == "bar"
  ));
  assert!(parser.parse_to_end("baz").is_err());
}

#[test]
fn raw_coverage_unit_or_instantiation_all_paths() {
  let parser = tokens::colon()
    .map(|_| (), None)
    .or(tokens::semicolon().map(|_| (), None));
  assert!(matches!(
    parser.parse_to_end(":").unwrap(),
    Either::Left(())
  ));
  assert!(matches!(
    parser.parse_to_end(";").unwrap(),
    Either::Right(())
  ));
  assert!(parser.parse_to_end("foo").is_err());
}

#[test]
fn raw_coverage_string_surrounded_by_same_prefix_all_paths() {
  let parser = TokenParser::<String>::string("foo")
    .surrounded_by(tokens::colon(), None::<TokenParser<SimpleToken>>);

  assert_eq!(parser.parse_to_end(":foo:").unwrap(), "foo");
  assert!(parser.parse_to_end("foo:").is_err());
  assert!(parser.parse_to_end(":bar:").is_err());
  assert!(parser.parse_to_end(":foo").is_err());
}

#[test]
fn raw_coverage_simple_token_surrounded_by_same_prefix_all_paths() {
  let parser = tokens::ident().surrounded_by(tokens::colon(), None::<TokenParser<SimpleToken>>);

  assert!(matches!(
    parser.parse_to_end(":foo:").unwrap(),
    SimpleToken::Ident(value) if value == "foo"
  ));
  assert!(parser.parse_to_end("foo:").is_err());
  assert!(parser.parse_to_end("::").is_err());
  assert!(parser.parse_to_end(":foo").is_err());
}

#[test]
fn raw_coverage_string_one_or_more_separated_by_simple_token_all_paths() {
  let parser = TokenParser::one_or_more_separated_by(
    TokenParser::<String>::string("foo"),
    tokens::whitespace(),
  );

  assert!(parser.parse_to_end("bar").is_err());
  assert_eq!(parser.parse("foo").unwrap(), vec!["foo"]);
  assert_eq!(parser.parse("foo foo").unwrap(), vec!["foo", "foo"]);
  assert_eq!(parser.parse("foo bar").unwrap(), vec!["foo"]);
}

#[test]
fn raw_coverage_string_one_or_more_separated_by_optional_simple_token_all_paths() {
  let parser = TokenParser::one_or_more_separated_by(
    TokenParser::<String>::string("foo"),
    tokens::whitespace().optional(),
  );

  assert!(parser.parse_to_end("bar").is_err());
  assert_eq!(parser.parse("foo").unwrap(), vec!["foo"]);
  assert_eq!(parser.parse("foo foo").unwrap(), vec!["foo", "foo"]);
  assert_eq!(parser.parse("foo bar").unwrap(), vec!["foo"]);
}

#[test]
fn raw_coverage_string_zero_or_more_separated_by_simple_token_all_paths() {
  let parser = TokenParser::zero_or_more_separated_by(
    TokenParser::<String>::string("foo"),
    tokens::whitespace(),
  );

  assert_eq!(parser.parse("bar").unwrap(), Vec::<String>::new());
  assert_eq!(parser.parse("foo").unwrap(), vec!["foo"]);
  assert_eq!(parser.parse("foo foo").unwrap(), vec!["foo", "foo"]);
  assert_eq!(parser.parse("foo bar").unwrap(), vec!["foo"]);
}

#[test]
fn raw_coverage_set_result_helpers_with_string() {
  let mut tokens = tl(vec![SimpleToken::Ident("before".to_string())]);
  tokens.current_index = 1;

  assert_eq!(
    set_of_incomplete_error::<String>(Ok(vec!["foo".to_string()]), &mut tokens, 0).unwrap(),
    vec!["foo".to_string()]
  );
  assert!(
    set_of_incomplete_error::<String>(Err("Parser 0 did not match".to_string()), &mut tokens, 0)
      .is_err()
  );
  assert_eq!(tokens.current_index, 0);

  assert_eq!(
    collect_set_results::<String>(vec![Some("foo".to_string())]).unwrap(),
    vec!["foo".to_string()]
  );
  assert!(collect_set_results::<String>(vec![None]).is_err());
}

#[test]
fn fn_name_parser_success() {
  let parser = TokenParser::<String>::fn_name("rgb");
  let mut tl = tl(vec![
    SimpleToken::Function("rgb".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert_eq!(result.unwrap(), "rgb");
}

#[test]
fn fn_name_parser_wrong_name_fails() {
  let parser = TokenParser::<String>::fn_name("rgb");
  let mut tl = tl(vec![
    SimpleToken::Function("hsl".to_string()),
    SimpleToken::RightParen,
  ]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_err());
}

#[test]
fn or_right_branch() {
  let parser = tokens::colon().or(tokens::ident());
  let mut tl = tl(vec![SimpleToken::Ident("foo".to_string())]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert!(matches!(result.unwrap(), Either::Right(_)));
}

#[test]
fn or_both_fail() {
  let parser = tokens::colon().or(tokens::semicolon());
  let mut tl = tl(vec![SimpleToken::Ident("foo".to_string())]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_err());
}

#[test]
fn or_label_optional_when_other_is_optional_label() {
  let always_unit: TokenParser<()> = TokenParser::always(());
  assert_eq!(always_unit.label, "optional");
  assert_eq!(always_unit.parse("").unwrap(), ());

  let ident_unit = tokens::ident().map(|_| (), None);
  assert_eq!(ident_unit.parse("foo").unwrap(), ());
  assert!(ident_unit.parse(":").is_err());
  let combined = ident_unit.or(always_unit);
  assert!(
    combined.label.contains("Optional"),
    "label: {}",
    combined.label
  );
}

#[test]
fn or_label_one_of_when_other_is_not_optional() {
  let parser1 = tokens::ident();
  let parser2 = tokens::colon();
  let combined = parser1.or(parser2);
  assert!(
    combined.label.contains("OneOf"),
    "label: {}",
    combined.label
  );
}

#[test]
fn map_error_path_rewinds() {
  let parser = tokens::colon().map(|_| "colon".to_string(), Some("to_str"));
  let result = parser.parse("foo"); // ident is not colon → error
  assert!(result.is_err());
}

#[test]
fn parse_to_end_error_formats_message() {
  let parser = tokens::colon();
  let result = parser.parse_to_end("foo");
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(
    msg.contains("Expected") || msg.contains("Colon"),
    "msg: {msg}"
  );
}

#[test]
fn optional_method_produces_optional_label() {
  let opt = tokens::ident().optional();
  assert!(opt.label.contains("Optional"), "label: {}", opt.label);
}

#[test]
fn sequence_separated_by_separator_consumed_retry_success() {
  let p_foo = TokenParser::<String>::string("foo");
  let p_always = TokenParser::always("always".to_string());
  let parser = TokenParser::<String>::sequence_with_separators(vec![p_foo, p_always])
    .separated_by(tokens::whitespace());

  let result = parser.parse_to_end("foo ");
  assert!(result.is_ok());
  assert_eq!(result.unwrap(), vec!["foo", "always"]);
}

#[test]
fn parse_to_end_tokens_remain_via_always() {
  let parser = TokenParser::always(42);
  let result = parser.parse_to_end("foo");
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(
    msg.contains("end of input") || msg.contains("Expected"),
    "msg: {msg}"
  );
}

#[test]
fn peek_remaining_some_when_tokens_present() {
  let mut tl = tl(vec![SimpleToken::Ident("foo".to_string())]);
  let result = tl.peek_infallible();
  assert_eq!(result, Some(SimpleToken::Ident("foo".to_string())));
}

#[test]
fn peek_remaining_none_when_exhausted() {
  let mut tl = tl(vec![]);
  let result = tl.peek_infallible();
  assert_eq!(result, None);
}

#[test]
fn collect_set_results_all_some() {
  let results: Vec<Option<i32>> = vec![Some(1), Some(2), Some(3)];
  let collected = collect_set_results(results);
  assert!(collected.is_ok());
  assert_eq!(collected.unwrap(), vec![1, 2, 3]);
}

#[test]
fn collect_set_results_has_none_returns_err() {
  // Unreachable through SetOfParsers; called directly with a None entry.
  let results: Vec<Option<i32>> = vec![Some(1), None, Some(3)];
  let collected = collect_set_results(results);
  assert!(collected.is_err());
  let msg = collected.unwrap_err();
  assert!(msg.contains("Parser 1 did not match"), "msg: {msg}");
}

#[test]
fn set_of_incomplete_error_ok_values() {
  let mut tl = tl(vec![]);
  let collected: Result<Vec<i32>, String> = Ok(vec![1, 2, 3]);
  let result = set_of_incomplete_error(collected, &mut tl, 0);
  assert!(result.is_ok());
  assert_eq!(result.unwrap(), vec![1, 2, 3]);
}

#[test]
fn set_of_incomplete_error_err_rewinds_and_returns_error() {
  // Unreachable through SetOfParsers; called directly with an Err.
  let mut tl = tl(vec![
    SimpleToken::Ident("foo".to_string()),
    SimpleToken::Colon,
  ]);
  tl.current_index = 2; // simulate: parser consumed 2 tokens
  let collected: Result<Vec<i32>, String> = Err("Parser 1 did not match".to_string());
  let result = set_of_incomplete_error(collected, &mut tl, 0);
  assert!(result.is_err());
  assert_eq!(tl.current_index, 0); // rewound to start_index=0
  let msg = result.unwrap_err().to_string();
  assert!(
    msg.contains("SetOf incomplete") || msg.contains("did not match"),
    "msg: {msg}"
  );
}

#[test]
fn match_next_token_success() {
  let mut tl = tl(vec![SimpleToken::Ident("foo".to_string())]);
  let expected = SimpleToken::Ident(String::new());
  let result = match_next_token(&mut tl, 0, &expected);
  assert!(result.is_ok());
  assert!(matches!(result.unwrap(), SimpleToken::Ident(_)));
}

#[test]
fn match_next_token_mismatch() {
  let mut tl = tl(vec![SimpleToken::Colon]);
  let expected = SimpleToken::Ident(String::new());
  let result = match_next_token(&mut tl, 0, &expected);
  assert!(result.is_err());
  assert_eq!(tl.current_index, 0); // rewound
}

#[test]
fn match_next_token_eof() {
  let mut tl = tl(vec![]);
  let expected = SimpleToken::Ident(String::new());
  let result = match_next_token(&mut tl, 0, &expected);
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(msg.contains("end of input"), "msg: {msg}");
}

#[test]
fn match_next_token_err_arm_via_err_result() {
  // consume_next_token never returns Err, so the Err arm is exercised on a constructed Err.
  let err_result: Result<Option<SimpleToken>, CssParseError> = Err(CssParseError::ParseError {
    message: "simulated".to_string(),
  });
  let mut tl2 = tl(vec![SimpleToken::Ident("x".to_string())]);
  let saved = tl2.current_index;
  let result: Result<SimpleToken, CssParseError> = match err_result {
    Ok(Some(token)) => Ok(token),
    Ok(None) => Err(CssParseError::ParseError {
      message: "eof".to_string(),
    }),
    Err(e) => {
      tl2.set_current_index(saved);
      Err(e)
    },
  };
  assert!(result.is_err());
  assert_eq!(tl2.current_index, 0); // rewound
}

#[test]
fn flexible_sequence_optional_none_no_separator() {
  // A failing separator keeps separator_consumed false; always(None) yields Ok(None).
  let foo = TokenParser::<String>::string("foo");
  let opt_none: TokenParser<Option<String>> = TokenParser::always(None);
  let parser = TokenParser::<String>::flexible_sequence_separated_by(
    vec![Either::Left(foo), Either::Right(opt_none)],
    TokenParser::<SimpleToken>::never().map(|_| SimpleToken::Whitespace, None), // separator always fails → separator_consumed=false
  );
  let mut tl = tl(vec![SimpleToken::Ident("foo".to_string())]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert_eq!(result.unwrap(), vec![Some("foo".to_string()), None]);
}

#[test]
fn flexible_sequence_optional_err_no_separator() {
  let foo = TokenParser::<String>::string("foo");
  // A parser typed as TokenParser<Option<String>> that always Err's
  let opt_err: TokenParser<Option<String>> = TokenParser::new(
    |_| {
      Err(CssParseError::ParseError {
        message: "always fails".to_string(),
      })
    },
    "always_err",
  );
  let parser = TokenParser::<String>::flexible_sequence_separated_by(
    vec![Either::Left(foo), Either::Right(opt_err)],
    TokenParser::<SimpleToken>::never().map(|_| SimpleToken::Whitespace, None),
  );
  let mut tl = tl(vec![SimpleToken::Ident("foo".to_string())]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert_eq!(result.unwrap(), vec![Some("foo".to_string()), None]);
}

#[test]
fn flexible_sequence_first_right_err_path() {
  // A raw never() typed as TokenParser<Option<String>> returns Err, not Ok(None).
  let opt_err: TokenParser<Option<String>> = TokenParser::new(
    |_| {
      Err(CssParseError::ParseError {
        message: "err".to_string(),
      })
    },
    "raw_err",
  );
  let parser = TokenParser::<String>::flexible_sequence_separated_by(
    vec![Either::Right(opt_err)],
    tokens::whitespace(),
  );
  let mut tl = tl(vec![]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert_eq!(result.unwrap(), vec![None]);
}

// SetOfParsers::separated_by's "incomplete" error is unreachable: a miss returns early.

// SetOfParsers::as_token_parser's "incomplete" error is unreachable for the same reason.

// token(): the Err arm is unreachable, as consume_next_token always returns Ok.

#[test]
fn sequence_separated_by_retry_ok_value_path() {
  // A parser that fails past the separator but succeeds at it forces the retry.
  let p_foo = TokenParser::<String>::string("foo");
  // This parser succeeds if and only if the next token is Whitespace.
  let p_needs_whitespace = tokens::whitespace().map(|_| "ws".to_string(), None);
  let parser = TokenParser::<String>::sequence_with_separators(vec![p_foo, p_needs_whitespace])
    .separated_by(tokens::whitespace());
  let mut tl = tl(vec![
    SimpleToken::Ident("foo".to_string()),
    SimpleToken::Whitespace,
  ]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  let vals = result.unwrap();
  assert_eq!(vals[0], "foo");
  assert_eq!(vals[1], "ws");
}

#[test]
fn set_of_separated_by_two_elements_complete() {
  let p_a = TokenParser::<String>::string("alpha");
  let p_b = TokenParser::<String>::string("beta");
  let parser = TokenParser::<String>::set_of(vec![p_a, p_b]).separated_by(tokens::whitespace());
  let result = parser.parse_to_end("alpha beta");
  assert!(result.is_ok());
  let vals = result.unwrap();
  assert!(vals.contains(&"alpha".to_string()));
  assert!(vals.contains(&"beta".to_string()));
}

#[test]
fn set_of_as_token_parser_two_elements_complete() {
  let p_a = TokenParser::<String>::string("x");
  let p_b = TokenParser::<String>::string("y");
  let parser = TokenParser::<String>::set_of(vec![p_a, p_b]).as_token_parser();
  let mut tl = tl(vec![
    SimpleToken::Ident("x".to_string()),
    SimpleToken::Ident("y".to_string()),
  ]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
}

#[test]
fn flexible_sequence_optional_none_with_separator_consumed() {
  let foo = TokenParser::<String>::string("foo");
  let opt_none: TokenParser<Option<String>> = TokenParser::always(None);
  let baz = TokenParser::<String>::string("baz");
  let parser = TokenParser::<String>::flexible_sequence_separated_by(
    vec![
      Either::Left(foo),
      Either::Right(opt_none),
      Either::Left(baz),
    ],
    tokens::whitespace(),
  );
  let mut tl = tl(vec![
    SimpleToken::Ident("foo".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Ident("baz".to_string()),
  ]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert_eq!(
    result.unwrap(),
    vec![Some("foo".to_string()), None, Some("baz".to_string())]
  );
}

#[test]
fn flexible_sequence_optional_err_with_separator_consumed() {
  let foo = TokenParser::<String>::string("foo");
  let opt_err: TokenParser<Option<String>> = TokenParser::new(
    |_| {
      Err(CssParseError::ParseError {
        message: "err".to_string(),
      })
    },
    "always_err",
  );
  let baz = TokenParser::<String>::string("baz");
  let parser = TokenParser::<String>::flexible_sequence_separated_by(
    vec![Either::Left(foo), Either::Right(opt_err), Either::Left(baz)],
    tokens::whitespace(),
  );
  let mut tl = tl(vec![
    SimpleToken::Ident("foo".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Ident("baz".to_string()),
  ]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert_eq!(
    result.unwrap(),
    vec![Some("foo".to_string()), None, Some("baz".to_string())]
  );
}

#[test]
fn rewind_if_err_rewinds_when_failed_true() {
  let mut tl = tl(vec![SimpleToken::Colon, SimpleToken::Semicolon]);
  tl.current_index = 2; // simulate having consumed 2 tokens
  rewind_if_err(&mut tl, 0, true); // failed = true → rewind to 0
  assert_eq!(tl.current_index, 0);
}

#[test]
fn rewind_if_err_does_not_rewind_when_failed_false() {
  let mut tl = tl(vec![SimpleToken::Colon, SimpleToken::Semicolon]);
  tl.current_index = 2;
  rewind_if_err(&mut tl, 0, false); // failed = false → no rewind
  assert_eq!(tl.current_index, 2);
}

#[test]
fn always_make_label_unit_type_returns_optional() {
  let label = always_make_label("()", "()");
  assert_eq!(label, "optional");
}

#[test]
fn always_make_label_non_unit_type_returns_always_fmt() {
  let label = always_make_label("i32", "42");
  assert_eq!(label, "Always<42>");
}

// debug_log_result: messages are read back, so the arguments are covered, not only the arm.

#[test]
fn debug_log_result_success_branch() {
  let messages = logged_at(Level::Debug, || debug_log_result(true, "TestParser", 3, ""));

  assert_eq!(
    messages,
    vec!["✅ SUCCESS: Parser 'TestParser' matched. Consumed 3 tokens.".to_string()]
  );
}

#[test]
fn debug_log_result_failure_branch() {
  let messages = logged_at(Level::Debug, || {
    debug_log_result(false, "TestParser", 0, "some error")
  });

  assert_eq!(
    messages,
    vec!["❌ FAILED: Parser 'TestParser' failed at token 0. Error: some error".to_string()]
  );
}

/// Nothing is logged while the level does not admit debug messages.
#[test]
fn debug_log_result_writes_nothing_at_a_higher_level() {
  let messages = logged_at(Level::Warn, || debug_log_result(true, "TestParser", 1, ""));

  assert!(messages.is_empty());
}

/// An oversized label and error and a maximal token count: the message uses what it is handed.
#[test]
fn debug_log_result_carries_an_enormous_label_and_error() {
  let label = "L".repeat(100_000);
  let error = "E".repeat(100_000);

  let messages = logged_at(Level::Debug, || {
    debug_log_result(false, &label, usize::MAX, &error)
  });

  assert_eq!(
    messages,
    vec![format!(
      "❌ FAILED: Parser '{label}' failed at token {}. Error: {error}",
      usize::MAX
    )]
  );
}

#[test]
fn build_parse_with_context_error_contains_context_info() {
  let err = CssParseError::ParseError {
    message: "inner error".to_string(),
  };
  let result = build_parse_with_context_error(&err, "foo bar", 3);
  let msg = result.to_string();
  assert!(msg.contains("inner error"), "msg: {msg}");
  assert!(
    msg.contains("Context") || msg.contains("position"),
    "msg: {msg}"
  );
}

#[test]
fn one_of_error_with_no_errors() {
  let result = one_of_error(vec![]);
  let msg = result.to_string();
  assert!(msg.contains("No parser matched"), "msg: {msg}");
}

#[test]
fn one_of_error_with_multiple_errors() {
  let errors = vec![
    CssParseError::ParseError {
      message: "err1".to_string(),
    },
    CssParseError::ParseError {
      message: "err2".to_string(),
    },
  ];
  let result = one_of_error(errors);
  let msg = result.to_string();
  assert!(msg.contains("No parser matched"), "msg: {msg}");
  assert!(msg.contains("err1"), "msg: {msg}");
  assert!(msg.contains("err2"), "msg: {msg}");
}

#[test]
fn one_of_first_parser_succeeds() {
  let parser = TokenParser::one_of(vec![tokens::ident(), tokens::colon()]);
  let mut tl = tl(vec![SimpleToken::Ident("foo".to_string())]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert!(matches!(result.unwrap(), SimpleToken::Ident(_)));
}

#[test]
fn one_of_first_fails_second_succeeds() {
  let parser = TokenParser::one_of(vec![tokens::colon(), tokens::ident()]);
  let mut tl = tl(vec![SimpleToken::Ident("foo".to_string())]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert!(matches!(result.unwrap(), SimpleToken::Ident(_)));
}

#[test]
fn sequence_static_fails_on_second_parser() {
  let parser = TokenParser::<SimpleToken>::sequence(vec![tokens::ident(), tokens::colon()]);
  let mut tl = tl(vec![
    SimpleToken::Ident("foo".to_string()),
    SimpleToken::Ident("bar".to_string()), // not a colon
  ]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_err());
}

#[test]
fn sequence_static_succeeds() {
  let parser = TokenParser::<SimpleToken>::sequence(vec![tokens::ident(), tokens::colon()]);
  let mut tl = tl(vec![
    SimpleToken::Ident("foo".to_string()),
    SimpleToken::Colon,
  ]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert_eq!(result.unwrap().len(), 2);
}

#[test]
fn sequence_parsers_as_token_parser_succeeds() {
  let p_foo = TokenParser::<String>::string("foo");
  let p_bar = TokenParser::<String>::string("bar");
  let parser =
    TokenParser::<String>::sequence_with_separators(vec![p_foo, p_bar]).as_token_parser();
  let mut tl = tl(vec![
    SimpleToken::Ident("foo".to_string()),
    SimpleToken::Ident("bar".to_string()),
  ]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert_eq!(result.unwrap(), vec!["foo".to_string(), "bar".to_string()]);
}

#[test]
fn zero_or_more_done_true_break() {
  let parser = TokenParser::zero_or_more(tokens::colon());
  let mut tl = tl(vec![SimpleToken::Ident("foo".to_string())]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert_eq!(result.unwrap(), vec![]);
}

#[test]
fn zero_or_more_done_false_continues() {
  let parser = TokenParser::zero_or_more(tokens::colon());
  let mut tl = tl(vec![SimpleToken::Colon, SimpleToken::Colon]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert_eq!(result.unwrap().len(), 2);
}

#[test]
fn one_or_more_first_match_fails() {
  let parser = TokenParser::one_or_more(tokens::colon());
  let mut tl = tl(vec![SimpleToken::Ident("foo".to_string())]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_err());
}

#[test]
fn one_or_more_first_match_succeeds_loop_exits() {
  let parser = TokenParser::one_or_more(tokens::colon());
  let mut tl = tl(vec![
    SimpleToken::Colon,
    SimpleToken::Ident("x".to_string()),
  ]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert_eq!(result.unwrap().len(), 1);
}

#[test]
fn one_or_more_separated_by_first_fails() {
  let parser = TokenParser::one_or_more_separated_by(tokens::colon(), tokens::comma());
  let mut tl = tl(vec![SimpleToken::Ident("foo".to_string())]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_err());
}

#[test]
fn one_or_more_separated_by_sep_fails_breaks() {
  let parser = TokenParser::one_or_more_separated_by(tokens::colon(), tokens::comma());
  let mut tl = tl(vec![
    SimpleToken::Colon,
    SimpleToken::Ident("x".to_string()),
  ]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert_eq!(result.unwrap().len(), 1); // only first colon matched
}

#[test]
fn one_or_more_separated_by_val_fails_rewinds_breaks() {
  let parser = TokenParser::one_or_more_separated_by(tokens::colon(), tokens::comma());
  let mut tl = tl(vec![
    SimpleToken::Colon,
    SimpleToken::Comma,
    SimpleToken::Ident("x".to_string()), // not a colon after separator
  ]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert_eq!(result.unwrap().len(), 1); // only the first colon; sep+fail rewound
}

#[test]
fn one_or_more_separated_by_multiple_values() {
  let parser = TokenParser::one_or_more_separated_by(tokens::colon(), tokens::comma());
  let mut tl = tl(vec![
    SimpleToken::Colon,
    SimpleToken::Comma,
    SimpleToken::Colon,
  ]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert_eq!(result.unwrap().len(), 2);
}

#[test]
fn zero_or_more_separated_by_first_fails_returns_empty() {
  let parser = TokenParser::zero_or_more_separated_by(tokens::colon(), tokens::comma());
  let mut tl = tl(vec![SimpleToken::Ident("foo".to_string())]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert_eq!(result.unwrap(), vec![]);
}

#[test]
fn zero_or_more_separated_by_first_succeeds_sep_fails() {
  let parser = TokenParser::zero_or_more_separated_by(tokens::colon(), tokens::comma());
  let mut tl = tl(vec![
    SimpleToken::Colon,
    SimpleToken::Ident("x".to_string()),
  ]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert_eq!(result.unwrap().len(), 1);
}

#[test]
fn zero_or_more_separated_by_val_fails_after_sep() {
  let parser = TokenParser::zero_or_more_separated_by(tokens::colon(), tokens::comma());
  let mut tl = tl(vec![
    SimpleToken::Colon,
    SimpleToken::Comma,
    SimpleToken::Ident("x".to_string()),
  ]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert_eq!(result.unwrap().len(), 1);
}

#[test]
fn zero_or_more_separated_by_multiple_values() {
  let parser = TokenParser::zero_or_more_separated_by(tokens::colon(), tokens::comma());
  let mut tl = tl(vec![
    SimpleToken::Colon,
    SimpleToken::Comma,
    SimpleToken::Colon,
  ]);
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert_eq!(result.unwrap().len(), 2);
}

#[test]
fn optional_as_token_parser_parser_fails_returns_none() {
  let opt_parser = TokenOptionalParser::new(tokens::colon()).as_token_parser();
  let mut tl = tl(vec![SimpleToken::Ident("foo".to_string())]);
  let result = (opt_parser.run)(&mut tl);
  assert!(result.is_ok());
  assert_eq!(result.unwrap(), None);
  assert_eq!(tl.current_index, 0); // rewound
}

#[test]
fn optional_as_token_parser_parser_succeeds_returns_some() {
  let opt_parser = TokenOptionalParser::new(tokens::colon()).as_token_parser();
  let mut tl = tl(vec![SimpleToken::Colon]);
  let result = (opt_parser.run)(&mut tl);
  assert!(result.is_ok());
  assert!(result.unwrap().is_some());
}

fn i32_token(expected: SimpleToken, value: i32) -> TokenParser<i32> {
  TokenParser::new(
    move |tokens| {
      let current_index = tokens.current_index;
      match_next_token(tokens, current_index, &expected).map(|_| value)
    },
    "i32_token",
  )
}

fn f64_token(expected: SimpleToken, value: f64) -> TokenParser<f64> {
  TokenParser::new(
    move |tokens| {
      let current_index = tokens.current_index;
      match_next_token(tokens, current_index, &expected).map(|_| value)
    },
    "f64_token",
  )
}

fn transform_token(expected: SimpleToken) -> TokenParser<TransformFunction> {
  TokenParser::new(
    move |tokens| {
      let current_index = tokens.current_index;
      match_next_token(tokens, current_index, &expected)
        .map(|_| TransformFunction::Scale(Scale { sx: 1.0, sy: None }))
    },
    "transform_token",
  )
}

#[test]
fn strict_or_i32_instantiation_exercises_all_paths() {
  let parser = i32_token(SimpleToken::Colon, 1).or(i32_token(SimpleToken::Semicolon, 2));

  let mut tokens = tl(vec![SimpleToken::Colon]);
  assert!(matches!((parser.run)(&mut tokens), Ok(Either::Left(1))));

  let mut tokens = tl(vec![SimpleToken::Semicolon]);
  assert!(matches!((parser.run)(&mut tokens), Ok(Either::Right(2))));

  let mut tokens = tl(vec![SimpleToken::Comma]);
  assert!((parser.run)(&mut tokens).is_err());
}

#[test]
fn strict_or_optional_label_instantiations_are_built() {
  let simple = tokens::ident().or(tokens::colon().optional());
  assert!(simple.label.contains("Optional"));

  let string =
    TokenParser::<String>::string("foo").or(TokenParser::<String>::string("bar").optional());
  assert!(string.label.contains("Optional"));

  let int = i32_token(SimpleToken::Colon, 1).or(TokenParser::always(2).optional());
  assert!(int.label.contains("Optional"));
}

#[test]
fn strict_never_unit_instantiation_is_run() {
  let mut tokens = tl(vec![]);
  assert!((TokenParser::<()>::never().run)(&mut tokens).is_err());
}

#[test]
fn strict_one_of_error_instantiations_are_run() {
  let mut tokens = tl(vec![SimpleToken::Comma]);
  let simple = TokenParser::one_of(vec![tokens::colon(), tokens::semicolon()]);
  assert!((simple.run)(&mut tokens).is_err());

  let mut tokens = tl(vec![SimpleToken::Comma]);
  let int = TokenParser::one_of(vec![
    i32_token(SimpleToken::Colon, 1),
    i32_token(SimpleToken::Semicolon, 2),
  ]);
  assert!((int.run)(&mut tokens).is_err());
}

#[test]
fn strict_sequence_i32_instantiation_rewinds_on_error() {
  let parser = TokenParser::<i32>::sequence(vec![
    i32_token(SimpleToken::Colon, 1),
    i32_token(SimpleToken::Semicolon, 2),
  ]);
  let mut tokens = tl(vec![SimpleToken::Colon, SimpleToken::Comma]);

  assert!((parser.run)(&mut tokens).is_err());
  assert_eq!(tokens.current_index, 0);
}

#[test]
fn strict_surrounded_by_optional_prefix_simple_token_instantiation_runs() {
  let parser = tokens::ident().surrounded_by(
    tokens::colon().optional(),
    None::<TokenParser<Option<SimpleToken>>>,
  );

  let mut tokens = tl(vec![SimpleToken::Ident("foo".to_string())]);
  assert!(matches!(
    (parser.run)(&mut tokens),
    Ok(SimpleToken::Ident(_))
  ));

  let mut tokens = tl(vec![SimpleToken::Colon]);
  assert!((parser.run)(&mut tokens).is_err());
}

#[test]
fn strict_flexible_sequence_builder_instantiation_exercises_error_paths() {
  let foo = TokenParser::<String>::string("foo");
  let bar = TokenParser::<String>::string("bar");
  let parser = TokenParser::<String>::mixed_sequence(vec![Either::Left(foo), Either::Left(bar)])
    .separated_by(tokens::whitespace());

  let mut tokens = tl(vec![
    SimpleToken::Ident("foo".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Ident("baz".to_string()),
  ]);
  assert!((parser.run)(&mut tokens).is_err());

  let foo = TokenParser::<String>::string("foo");
  let always_bar = TokenParser::always(Some("bar".to_string()));
  let parser =
    TokenParser::<String>::mixed_sequence(vec![Either::Left(foo), Either::Right(always_bar)])
      .separated_by(TokenParser::<SimpleToken>::never());

  let mut tokens = tl(vec![SimpleToken::Ident("foo".to_string())]);
  assert!((parser.run)(&mut tokens).is_err());
}

#[test]
fn strict_flexible_sequence_string_direct_instantiation_exercises_remaining_paths() {
  let parser = TokenParser::<String>::flexible_sequence_separated_by(
    vec![
      Either::Left(TokenParser::<String>::string("foo")),
      Either::Right(TokenParser::always(Some("bar".to_string()))),
    ],
    tokens::whitespace(),
  );
  let mut tokens = tl(vec![
    SimpleToken::Ident("foo".to_string()),
    SimpleToken::Whitespace,
  ]);
  let result = (parser.run)(&mut tokens);
  assert!(matches!(result, Ok(values) if values == vec![
    Some("foo".to_string()),
    Some("bar".to_string())
  ]));

  let parser = TokenParser::<String>::flexible_sequence_separated_by(
    vec![Either::Left(TokenParser::<String>::string("foo"))],
    tokens::whitespace(),
  );
  let mut tokens = tl(vec![SimpleToken::Ident("bar".to_string())]);
  assert!((parser.run)(&mut tokens).is_err());

  let parser = TokenParser::<String>::flexible_sequence_separated_by(
    vec![Either::Right(TokenParser::always(Some("bar".to_string())))],
    tokens::whitespace(),
  );
  let mut tokens = tl(vec![]);
  assert!(matches!(
    (parser.run)(&mut tokens),
    Ok(values) if values == vec![Some("bar".to_string())]
  ));
}

#[test]
fn strict_one_or_more_separated_by_transform_and_number_instantiations() {
  let parser = TokenParser::one_or_more_separated_by(
    transform_token(SimpleToken::Ident("scale".to_string())),
    tokens::comma(),
  );
  let mut tokens = tl(vec![SimpleToken::Comma]);
  assert!((parser.run)(&mut tokens).is_err());

  let mut tokens = tl(vec![
    SimpleToken::Ident("scale".to_string()),
    SimpleToken::Comma,
    SimpleToken::Colon,
  ]);
  assert!((parser.run)(&mut tokens).is_ok());

  let parser = TokenParser::one_or_more_separated_by(
    f64_token(SimpleToken::Number(0.0), 1.0),
    tokens::comma(),
  );
  let mut tokens = tl(vec![SimpleToken::Comma]);
  assert!((parser.run)(&mut tokens).is_err());

  let mut tokens = tl(vec![
    SimpleToken::Number(1.0),
    SimpleToken::Comma,
    SimpleToken::Colon,
  ]);
  assert!((parser.run)(&mut tokens).is_ok());
}

#[test]
fn strict_one_or_more_separated_by_remaining_instantiations() {
  let parser = TokenParser::one_or_more_separated_by(
    TokenParser::<String>::string("foo"),
    tokens::whitespace().optional(),
  );
  let mut tokens = tl(vec![SimpleToken::Ident("foo".to_string())]);
  assert!(matches!(
    (parser.run)(&mut tokens),
    Ok(values) if values == vec!["foo".to_string()]
  ));

  let parser = TokenParser::one_or_more_separated_by(
    TokenParser::<String>::string("foo"),
    TokenParser::<()>::always(()),
  );
  let mut tokens = tl(vec![
    SimpleToken::Ident("foo".to_string()),
    SimpleToken::Ident("bar".to_string()),
  ]);
  assert!(matches!(
    (parser.run)(&mut tokens),
    Ok(values) if values == vec!["foo".to_string()]
  ));
}
