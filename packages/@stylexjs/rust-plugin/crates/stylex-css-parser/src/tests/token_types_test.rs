/*!
Token types tests.
*/

use crate::token_types::TokenList;

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
  fn test_token_list_restore_invalid_position() {
    let mut list = TokenList::new("a");
    let result = list.restore_position(999);
    assert!(result.is_err());
  }
}
