use super::*;
use crate::token_types::TokenList;

fn token_list(tokens: Vec<SimpleToken>) -> TokenList {
  TokenList {
    tokens,
    current_index: 0,
    depth: 0,
  }
}

// ── VerticalKeyword::as_str center arm ──

#[test]
fn vertical_keyword_as_str_center() {
  assert_eq!(VerticalKeyword::Center.as_str(), "center");
}

// ── Position::is_horizontal_ident — non-Ident branch ──

#[test]
fn is_horizontal_ident_returns_false_for_non_ident() {
  let result = Position::is_horizontal_ident(&SimpleToken::Whitespace);
  assert!(!result);
}

// ── Position::token_to_horizontal_keyword — unreachable arms ──

#[test]
#[should_panic]
fn token_to_horizontal_keyword_wildcard_panics() {
  // Exercises `_ => stylex_unreachable!()`.
  Position::token_to_horizontal_keyword(SimpleToken::Ident("top".to_string()));
}

#[test]
#[should_panic]
fn token_to_horizontal_keyword_non_ident_panics() {
  // Exercises `else { stylex_unreachable!() }`.
  Position::token_to_horizontal_keyword(SimpleToken::Whitespace);
}

// ── Position::is_vertical_ident — non-Ident branch ──

#[test]
fn is_vertical_ident_returns_false_for_non_ident() {
  let result = Position::is_vertical_ident(&SimpleToken::Whitespace);
  assert!(!result);
}

// ── Position::token_to_vertical_keyword — unreachable arms ──

#[test]
#[should_panic]
fn token_to_vertical_keyword_wildcard_panics() {
  // Exercises `_ => stylex_unreachable!()`.
  Position::token_to_vertical_keyword(SimpleToken::Ident("left".to_string()));
}

#[test]
#[should_panic]
fn token_to_vertical_keyword_non_ident_panics() {
  // Exercises `else { stylex_unreachable!() }`.
  Position::token_to_vertical_keyword(SimpleToken::Whitespace);
}

// ── Vertical-then-horizontal parser ──

#[test]
fn both_keywords_rewinds_when_vertical_first_second_component_fails() {
  let mut tokens = token_list(vec![
    SimpleToken::Ident("top".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Ident("bottom".to_string()),
  ]);

  let result = Position::parse_both_keywords(&mut tokens);

  assert!(result.is_err());
  assert_eq!(tokens.current_index, 0);
}

#[test]
fn parses_length_plus_horizontal() {
  assert!(Position::parser().parse_to_end("10px left").is_err());
}

// ── position_parser convenience fn ──

#[test]
fn position_parser_fn_works() {
  let result = position_parser().parse_to_end("left").unwrap();
  assert!(matches!(
    result.horizontal,
    Some(Horizontal::Keyword(HorizontalKeyword::Left))
  ));
}

#[test]
fn parses_vertical_keyword_plus_horizontal_length() {
  let result = Position::parser().parse_to_end("top 10px").unwrap();

  assert!(matches!(result.horizontal, Some(Horizontal::Length(_))));
  assert!(matches!(
    result.vertical,
    Some(Vertical::Keyword(VerticalKeyword::Top))
  ));
}

#[test]
fn vertical_keyword_plus_horizontal_length_rewinds_when_length_fails() {
  let mut tokens = token_list(vec![
    SimpleToken::Ident("top".to_string()),
    SimpleToken::Whitespace,
    SimpleToken::Ident("left".to_string()),
  ]);

  let result = Position::parse_vertical_keyword_plus_horizontal_length(&mut tokens);

  assert!(result.is_err());
  assert_eq!(tokens.current_index, 0);
}
