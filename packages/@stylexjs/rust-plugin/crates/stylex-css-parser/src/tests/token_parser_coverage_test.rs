use super::*;
use crate::{
  capturing_logger::logged_at,
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
fn tokens_string_success() {
  let mut tl = tl(vec![SimpleToken::String("hello".to_string())]);
  let parser = tokens::string();
  let result = (parser.run)(&mut tl);
  assert!(result.is_ok());
  assert!(matches!(result.unwrap(), SimpleToken::String(_)));
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
#[should_panic]
fn extract_ident_value_unreachable_arm() {
  extract_ident_value(SimpleToken::Colon);
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
fn token_parser_tokens_accessor() {
  let _tokens = TokenParser::<SimpleToken>::tokens();
  let parser = TokenParser::<SimpleToken>::tokens();
  let ident_parser = Tokens::ident(); // just ensure Tokens is usable
  let result = ident_parser.parse("foo");
  assert!(result.is_ok());
  let _ = parser; // tokens() returns Tokens
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
