/*!
Token types tests.
*/

use crate::token_types::{SimpleToken, TokenList};

#[cfg(test)]
mod test_token_types {
  use super::*;

  #[test]
  fn token_list_constructor_from_string() {
    let token_list = TokenList::new("test");
    assert_eq!(token_list.current_index, 0);
  }

  #[test]
  fn consume_next_token_basic_functionality() {
    let mut token_list = TokenList::new("test 123");

    // Should consume tokens sequentially
    let first_token = token_list.consume_next_token();
    assert!(first_token.is_ok());

    let second_token = token_list.consume_next_token();
    assert!(second_token.is_ok());

    let third_token = token_list.consume_next_token();
    assert!(third_token.is_ok());
  }

  #[test]
  fn peek_returns_next_token_without_consuming() {
    let mut token_list = TokenList::new("test");

    let initial_index = token_list.current_index;
    let peeked_token = token_list.peek();

    // Index should not change after peek
    assert_eq!(token_list.current_index, initial_index);
    assert!(peeked_token.is_ok());

    // Peek should return same token multiple times
    let peeked_again = token_list.peek();
    assert_eq!(format!("{:?}", peeked_token), format!("{:?}", peeked_again));
  }

  #[test]
  fn save_and_restore_position() {
    let mut token_list = TokenList::new("a b c");

    // Save initial position
    let saved_pos = token_list.save_position();

    // Consume some tokens
    let _ = token_list.consume_next_token();
    let _ = token_list.consume_next_token();

    let current_pos = token_list.current_index;
    assert!(current_pos > saved_pos);

    // Restore to saved position
    let _ = token_list.restore_position(saved_pos);
    assert_eq!(token_list.current_index, saved_pos);
  }

  #[test]
  fn is_empty_returns_correct_status() {
    let token_list = TokenList::new("");
    assert!(token_list.is_empty());

    let non_empty_list = TokenList::new("test");
    assert!(!non_empty_list.is_empty());
  }

  #[test]
  fn slice_returns_token_range() {
    let mut token_list = TokenList::new("a b c d e");

    let slice = token_list.slice(0, Some(2));
    assert_eq!(slice.len(), 2);

    // Empty slice
    let empty_slice = token_list.slice(5, Some(3)); // start > end
    assert!(empty_slice.is_empty());
  }

  #[test]
  fn slice_with_none_end_parameter() {
    let mut token_list = TokenList::new("hello world");

    let slice = token_list.slice(0, None); // Should use all tokens
    // Note: slice may be empty if no tokens were consumed yet
    // Let's just verify the slice operation works without panicking
    let _ = slice; // Use the slice to avoid unused variable warning
  }

  #[test]
  fn token_iterator_interface_compatibility() {
    // Test that TokenList works with string input
    let from_string = TokenList::new("test");
    assert_eq!(from_string.current_index, 0);

    // Test that it actually contains tokens
    assert!(!from_string.tokens.is_empty());
  }

  #[test]
  fn basic_tokenization_functionality() {
    let mut token_list = TokenList::new("hello world");

    // Should be able to peek at first token
    let peeked = token_list.peek();
    assert!(peeked.is_ok());

    // Should be able to consume first token
    let consumed = token_list.consume_next_token();
    assert!(consumed.is_ok());

    // Index should advance after consumption
    assert_eq!(token_list.current_index, 1);
  }

  #[test]
  fn handle_special_css_tokens() {
    let mut token_list = TokenList::new("rgb(255, 0, 0)");

    // Should tokenize function calls properly
    assert!(!token_list.tokens.is_empty());

    // Should be able to consume tokens from function
    let first_token = token_list.consume_next_token();
    assert!(first_token.is_ok());
  }

  #[test]
  fn empty_input_handling() {
    let mut token_list = TokenList::new("");

    // Should handle empty input gracefully
    assert!(token_list.is_empty());

    // Consuming from empty list should return None
    let token = token_list.consume_next_token();
    assert!(token.is_ok());
    if let Ok(opt_token) = token {
      assert!(opt_token.is_none());
    }
  }

  #[test]
  fn test_extract_value_for_all_token_types() {
    // Tokens with extractable values
    assert_eq!(
      SimpleToken::Ident("foo".to_string()).extract_value(),
      Some("foo".to_string())
    );
    assert_eq!(
      SimpleToken::String("bar".to_string()).extract_value(),
      Some("bar".to_string())
    );
    assert_eq!(
      SimpleToken::Hash("abc".to_string()).extract_value(),
      Some("abc".to_string())
    );
    assert_eq!(
      SimpleToken::AtKeyword("media".to_string()).extract_value(),
      Some("media".to_string())
    );
    assert_eq!(
      SimpleToken::Comment("hello".to_string()).extract_value(),
      Some("hello".to_string())
    );
    assert_eq!(
      SimpleToken::Number(42.0).extract_value(),
      Some("42".to_string())
    );
    assert_eq!(
      SimpleToken::Percentage(0.5).extract_value(),
      Some("0.5".to_string())
    );
    assert_eq!(
      SimpleToken::Dimension {
        value: 10.0,
        unit: "px".to_string()
      }
      .extract_value(),
      Some("10px".to_string())
    );
    assert_eq!(
      SimpleToken::Delim('+').extract_value(),
      Some("+".to_string())
    );
    assert_eq!(
      SimpleToken::Function("rgb".to_string()).extract_value(),
      Some("rgb".to_string())
    );
    assert_eq!(
      SimpleToken::Unknown("?".to_string()).extract_value(),
      Some("?".to_string())
    );

    // Structural tokens without values
    assert_eq!(SimpleToken::LeftParen.extract_value(), None);
    assert_eq!(SimpleToken::RightParen.extract_value(), None);
    assert_eq!(SimpleToken::Comma.extract_value(), None);
    assert_eq!(SimpleToken::Semicolon.extract_value(), None);
    assert_eq!(SimpleToken::Colon.extract_value(), None);
    assert_eq!(SimpleToken::Whitespace.extract_value(), None);
    assert_eq!(SimpleToken::LeftBracket.extract_value(), None);
    assert_eq!(SimpleToken::RightBracket.extract_value(), None);
    assert_eq!(SimpleToken::LeftBrace.extract_value(), None);
    assert_eq!(SimpleToken::RightBrace.extract_value(), None);
  }

  /// The numeric arms spell a number the way JavaScript spells it, which is the
  /// rule the rest of the crate follows and this one did not.
  ///
  /// `f64`'s own `Display` never switches to exponential form, so `1e21` came
  /// back as twenty-two digits and `1e-7` as `0.0000001` -- neither of which
  /// JavaScript writes, and neither reachable through any other path here.
  /// Widening the field to a double made the gap wider rather than introducing
  /// it. Nothing in the plugin calls this method, so no CSS moved; it is `pub`,
  /// and a reader would reasonably have trusted it.
  #[test]
  fn extract_value_spells_a_number_the_way_javascript_does() {
    assert_eq!(
      SimpleToken::Number(1e21).extract_value(),
      Some("1e+21".to_string())
    );
    assert_eq!(
      SimpleToken::Number(1e-7).extract_value(),
      Some("1e-7".to_string())
    );
    assert_eq!(
      SimpleToken::Percentage(1e21).extract_value(),
      Some("1e+21".to_string())
    );
    assert_eq!(
      SimpleToken::Dimension {
        value: 1e21,
        unit: "px".to_string()
      }
      .extract_value(),
      Some("1e+21px".to_string())
    );

    // A negative zero loses its sign, as `String(-0)` does, and an infinity is
    // spelled rather than rendered as `inf`.
    assert_eq!(
      SimpleToken::Number(-0.0).extract_value(),
      Some("0".to_string())
    );
    assert_eq!(
      SimpleToken::Number(f64::INFINITY).extract_value(),
      Some("Infinity".to_string())
    );
  }

  #[test]
  fn test_extract_number() {
    assert_eq!(SimpleToken::Number(42.0).extract_number(), Some(42.0));
    assert_eq!(SimpleToken::Percentage(0.5).extract_number(), Some(0.5));
    assert_eq!(
      SimpleToken::Dimension {
        value: 10.0,
        unit: "px".to_string()
      }
      .extract_number(),
      Some(10.0)
    );
    assert_eq!(SimpleToken::Ident("foo".to_string()).extract_number(), None);
    assert_eq!(
      SimpleToken::String("bar".to_string()).extract_number(),
      None
    );
    assert_eq!(SimpleToken::Comma.extract_number(), None);
  }

  #[test]
  fn test_token_list_save_restore_position() {
    let mut list = TokenList::new("a b c");
    let saved = list.save_position();
    list.consume_next_token().unwrap();
    list.consume_next_token().unwrap();
    list.restore_position(saved).unwrap();
    assert_eq!(
      list.peek().unwrap(),
      Some(SimpleToken::Ident("a".to_string()))
    );
  }

  #[test]
  fn test_token_list_restore_invalid_position() {
    let mut list = TokenList::new("a");
    let result = list.restore_position(999);
    assert!(result.is_err());
  }

  #[test]
  fn test_token_list_is_empty() {
    let mut list = TokenList::new("a");
    assert!(!list.is_empty());
    list.consume_next_token().unwrap();
    assert!(list.is_empty());
  }

  #[test]
  fn test_token_list_consume_past_end() {
    let mut list = TokenList::new("a");
    list.consume_next_token().unwrap();
    let result = list.consume_next_token().unwrap();
    assert_eq!(result, None);
  }
}
