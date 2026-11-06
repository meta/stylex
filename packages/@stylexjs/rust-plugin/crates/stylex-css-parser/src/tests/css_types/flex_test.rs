/*!
CSS Flex Tests

Test CSS flex type with fr unit.
*/

#[cfg(test)]
mod flex_parse {
  use crate::css_types::flex::Flex;

  #[test]
  fn parses_valid_fr_values() {
    let result = Flex::parser().parse_to_end("1fr").unwrap();
    assert_eq!(result.fraction, 1.0);

    let result = Flex::parser().parse_to_end("2.5fr").unwrap();
    assert_eq!(result.fraction, 2.5);

    let result = Flex::parser().parse_to_end("0fr").unwrap();
    assert_eq!(result.fraction, 0.0);
  }

  #[test]
  fn rejects_invalid_fr_values() {
    assert!(Flex::parser().parse_to_end("-1fr").is_err());
    assert!(Flex::parser().parse_to_end("1 fr").is_err());
    assert!(Flex::parser().parse_to_end("1px").is_err());
  }
}
