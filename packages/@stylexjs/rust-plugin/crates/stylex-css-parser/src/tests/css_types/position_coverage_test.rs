use super::*;
use crate::token_types::TokenList;

fn token_list(tokens: Vec<SimpleToken>) -> TokenList {
  TokenList {
    tokens,
    current_index: 0,
    depth: 0,
  }
}

// ── VerticalKeyword::as_str center arm ─────────────────────────

#[test]
fn vertical_keyword_as_str_center() {
  // "center" arm of VerticalKeyword::as_str was not covered by existing tests.
  assert_eq!(VerticalKeyword::Center.as_str(), "center");
}

// ── Position::is_horizontal_ident — non-Ident branch ──────────

#[test]
fn is_horizontal_ident_returns_false_for_non_ident() {
  // Exercises the `else { false }` branch with a non-Ident token.
  let result = Position::is_horizontal_ident(&SimpleToken::Whitespace);
  assert!(!result);
}

#[test]
fn is_horizontal_ident_returns_true_for_valid_idents() {
  assert!(Position::is_horizontal_ident(&SimpleToken::Ident(
    "left".to_string()
  )));
  assert!(Position::is_horizontal_ident(&SimpleToken::Ident(
    "center".to_string()
  )));
  assert!(Position::is_horizontal_ident(&SimpleToken::Ident(
    "right".to_string()
  )));
}

#[test]
fn is_horizontal_ident_returns_false_for_unknown_ident() {
  assert!(!Position::is_horizontal_ident(&SimpleToken::Ident(
    "top".to_string()
  )));
}

// ── Position::token_to_horizontal_keyword — unreachable arms ─────────────

#[test]
fn token_to_horizontal_keyword_happy_path() {
  assert_eq!(
    Position::token_to_horizontal_keyword(SimpleToken::Ident("left".to_string())),
    HorizontalKeyword::Left
  );
  assert_eq!(
    Position::token_to_horizontal_keyword(SimpleToken::Ident("center".to_string())),
    HorizontalKeyword::Center
  );
  assert_eq!(
    Position::token_to_horizontal_keyword(SimpleToken::Ident("right".to_string())),
    HorizontalKeyword::Right
  );
}

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

// ── Position::is_vertical_ident — non-Ident branch ───────────

#[test]
fn is_vertical_ident_returns_false_for_non_ident() {
  let result = Position::is_vertical_ident(&SimpleToken::Whitespace);
  assert!(!result);
}

#[test]
fn is_vertical_ident_returns_true_for_valid_idents() {
  assert!(Position::is_vertical_ident(&SimpleToken::Ident(
    "top".to_string()
  )));
  assert!(Position::is_vertical_ident(&SimpleToken::Ident(
    "center".to_string()
  )));
  assert!(Position::is_vertical_ident(&SimpleToken::Ident(
    "bottom".to_string()
  )));
}

#[test]
fn is_vertical_ident_returns_false_for_unknown_ident() {
  assert!(!Position::is_vertical_ident(&SimpleToken::Ident(
    "left".to_string()
  )));
}

// ── Position::token_to_vertical_keyword — unreachable arms ───────────────

#[test]
fn token_to_vertical_keyword_happy_path() {
  assert_eq!(
    Position::token_to_vertical_keyword(SimpleToken::Ident("top".to_string())),
    VerticalKeyword::Top
  );
  assert_eq!(
    Position::token_to_vertical_keyword(SimpleToken::Ident("center".to_string())),
    VerticalKeyword::Center
  );
  assert_eq!(
    Position::token_to_vertical_keyword(SimpleToken::Ident("bottom".to_string())),
    VerticalKeyword::Bottom
  );
}

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

// ── Vertical-then-horizontal parser ──────────────────────

#[test]
fn parses_top_left_vertical_then_horizontal() {
  // "top left" exercises the vertical_horizontal_keywords branch.
  let result = Position::parser().parse_to_end("top left").unwrap();
  assert!(matches!(
    result.horizontal,
    Some(Horizontal::Keyword(HorizontalKeyword::Left))
  ));
  assert!(matches!(
    result.vertical,
    Some(Vertical::Keyword(VerticalKeyword::Top))
  ));
}

#[test]
fn parses_bottom_right_vertical_then_horizontal() {
  let result = Position::parser().parse_to_end("bottom right").unwrap();
  assert!(matches!(
    result.horizontal,
    Some(Horizontal::Keyword(HorizontalKeyword::Right))
  ));
  assert!(matches!(
    result.vertical,
    Some(Vertical::Keyword(VerticalKeyword::Bottom))
  ));
}

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

#[test]
fn numbers_only_rewinds_trailing_whitespace_without_second_length() {
  let mut tokens = token_list(vec![
    SimpleToken::Dimension {
      value: 10.0,
      unit: "px".to_string(),
    },
    SimpleToken::Whitespace,
  ]);

  let result = Position::parse_numbers_only(&mut tokens).unwrap();

  assert!(matches!(result.horizontal, Some(Horizontal::Length(_))));
  assert!(matches!(result.vertical, Some(Vertical::Length(_))));
  assert_eq!(tokens.current_index, 1);
}

// ── position_parser convenience fn ───────────────────────

#[test]
fn position_parser_fn_works() {
  // Exercises the `position_parser()` free function.
  let result = position_parser().parse_to_end("left").unwrap();
  assert!(matches!(
    result.horizontal,
    Some(Horizontal::Keyword(HorizontalKeyword::Left))
  ));
}

#[test]
fn position_parser_fn_parses_top() {
  let result = position_parser().parse_to_end("top").unwrap();
  assert!(matches!(
    result.vertical,
    Some(Vertical::Keyword(VerticalKeyword::Top))
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
fn vertical_keyword_plus_horizontal_length_requires_whitespace() {
  let mut tokens = token_list(vec![SimpleToken::Ident("top".to_string())]);

  let result = Position::parse_vertical_keyword_plus_horizontal_length(&mut tokens);

  assert!(result.is_err());
  assert_eq!(tokens.current_index, 0);
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
