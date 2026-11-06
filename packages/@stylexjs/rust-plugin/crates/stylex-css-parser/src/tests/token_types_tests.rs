// Tests extracted for token_types.rs behaviors and edge cases.
// Source: crates/stylex-css-parser/src/token_types.rs

use super::*;

#[test]
fn test_basic_tokenization() {
  let mut list = TokenList::new("color: red;\nbackground: rgb(1, 2, 3)");
  assert!(list.peek().unwrap().is_some());
  assert!(!list.get_all_tokens().is_empty());
}

#[test]
fn test_token_list_basic_peek_consume() {
  let mut list = TokenList::new("color: red;");
  let first = list.peek().unwrap();
  assert_eq!(first, Some(SimpleToken::Ident("color".to_string())));
  let consumed = list.consume_next_token().unwrap();
  assert_eq!(consumed, Some(SimpleToken::Ident("color".to_string())));
  let second = list.peek().unwrap();
  assert_eq!(second, Some(SimpleToken::Colon));
}

#[test]
fn test_rewind_and_slice() {
  let mut list = TokenList::new("a : b ; c");
  list.consume_next_token().unwrap(); // a
  list.consume_next_token().unwrap(); // :
  list.rewind(1);
  // With cssparser-backed tokenizer, whitespace tokens are preserved
  assert_eq!(list.peek().unwrap(), Some(SimpleToken::Whitespace));

  let slice = list.slice(1, Some(4));
  assert_eq!(slice.len(), 3);
  // slice should include whitespace, then colon, then whitespace
  assert_eq!(slice[0], SimpleToken::Whitespace);
  assert_eq!(slice[1], SimpleToken::Colon);
}
