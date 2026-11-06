/*!
Token types tests.
*/

use crate::token_types::{SimpleToken, TokenList};

#[cfg(test)]
mod test_token_types {
  use super::*;

  #[test]
  fn empty_input_handling() {
    let mut token_list = TokenList::new("");

    assert!(token_list.is_empty());

    let token = token_list.consume_next_token();
    assert!(token.is_ok());
    if let Ok(opt_token) = token {
      assert!(opt_token.is_none());
    }
  }

  #[test]
  fn test_extract_value_for_all_token_types() {
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
  fn test_token_list_restore_invalid_position() {
    let mut list = TokenList::new("a");
    let result = list.restore_position(999);
    assert!(result.is_err());
  }
}
