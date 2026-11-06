// Tests extracted for token_parser.rs behaviors and edge cases.
// Source: crates/stylex-css-parser/src/token_parser.rs

use super::*;

#[test]
fn test_always_parser() {
  let parser = TokenParser::always(42);
  let result = parser.parse("anything").unwrap();
  assert_eq!(result, 42);
}

#[test]
fn test_never_parser() {
  let parser: TokenParser<i32> = TokenParser::never();
  assert!(parser.parse("anything").is_err());
}

#[test]
fn test_map_parser() {
  let parser = TokenParser::always(10).map(|x| x * 2, Some("double"));
  let result = parser.parse("anything").unwrap();
  assert_eq!(result, 20);
}

#[test]
fn test_flat_map_parser() {
  let parser = TokenParser::always(5).flat_map(|x| TokenParser::always(x + 1), Some("add_one"));
  let result = parser.parse("anything").unwrap();
  assert_eq!(result, 6);
}

#[test]
fn test_optional_parser() {
  let success_parser = TokenParser::always(42).optional();
  let result = success_parser.parse("anything").unwrap();
  assert_eq!(result, Some(42));

  let fail_parser: TokenParser<Option<i32>> = TokenParser::<i32>::never().optional();
  let result = fail_parser.parse("anything").unwrap();
  assert_eq!(result, None);
}

#[test]
fn test_where_predicate_parser() {
  let parser = TokenParser::always(10).where_predicate(|&x| x > 5, Some("greater_than_5"));
  let result = parser.parse("anything").unwrap();
  assert_eq!(result, 10);

  let parser = TokenParser::always(3).where_predicate(|&x| x > 5, Some("greater_than_5"));
  assert!(parser.parse("anything").is_err());
}

#[test]
fn test_one_of_parser() {
  let parser = TokenParser::one_of(vec![
    TokenParser::<i32>::never(),
    TokenParser::always(42),
    TokenParser::always(24),
  ]);
  let result = parser.parse("anything").unwrap();
  assert_eq!(result, 42); // Should return first successful result
}

#[test]
fn test_sequence_parser() {
  let parser = TokenParser::<i32>::sequence(vec![
    TokenParser::always(1),
    TokenParser::always(2),
    TokenParser::always(3),
  ]);
  let result = parser.parse("anything").unwrap();
  assert_eq!(result, vec![1, 2, 3]);
}

#[test]
fn test_or_parser() {
  let parser1 = TokenParser::always(1);
  let parser2 = TokenParser::always(2);
  let combined = parser1.or(parser2);

  let result = combined.parse("anything").unwrap();
  assert!(matches!(result, Either::Left(1)));
}

#[test]
fn test_parse_to_end() {
  let parser = TokenParser::always(42);
  // This should work since always parser doesn't consume tokens
  let result = parser.parse_to_end("").unwrap();
  assert_eq!(result, 42);
}

#[test]
fn test_label_preservation() {
  let parser = TokenParser::always(42);
  assert!(parser.label.contains("Always"));

  let mapped = parser.map(|x| x * 2, Some("double"));
  assert!(mapped.label.contains("map(double)"));
  assert_eq!(mapped.parse("").unwrap(), 84);
}
