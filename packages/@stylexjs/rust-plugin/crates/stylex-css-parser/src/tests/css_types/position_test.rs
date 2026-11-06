/*!
CSS Position Tests

Test CSS Type: <position>
*/

#[cfg(test)]
mod test_css_type_position {
  use crate::css_types::position::{
    Horizontal, HorizontalKeyword, Position, Vertical, VerticalKeyword,
  };

  #[test]
  fn parses_single_keywords() {
    let result = Position::parser().parse_to_end("left").unwrap();
    assert!(matches!(
      result.horizontal,
      Some(Horizontal::Keyword(HorizontalKeyword::Left))
    ));
    assert!(result.vertical.is_none());

    let result = Position::parser().parse_to_end("right").unwrap();
    assert!(matches!(
      result.horizontal,
      Some(Horizontal::Keyword(HorizontalKeyword::Right))
    ));
    assert!(result.vertical.is_none());

    let result = Position::parser().parse_to_end("center").unwrap();
    assert!(result.horizontal.is_some() || result.vertical.is_some());

    let result = Position::parser().parse_to_end("top").unwrap();
    assert!(result.horizontal.is_none());
    assert!(matches!(
      result.vertical,
      Some(Vertical::Keyword(VerticalKeyword::Top))
    ));

    let result = Position::parser().parse_to_end("bottom").unwrap();
    assert!(result.horizontal.is_none());
    assert!(matches!(
      result.vertical,
      Some(Vertical::Keyword(VerticalKeyword::Bottom))
    ));
  }

  #[test]
  fn parses_keyword_with_length_percentage() {
    assert!(Position::parser().parse_to_end("left 50% top 20px").is_ok());

    assert!(Position::parser().parse_to_end("right 20px").is_ok());

    assert!(Position::parser().parse_to_end("50% top").is_ok());

    assert!(Position::parser().parse_to_end("30px bottom").is_ok());
  }

  #[test]
  fn rejects_invalid_positions() {
    assert!(Position::parser().parse_to_end("invalid").is_err());
    assert!(Position::parser().parse_to_end("left left").is_err());
    assert!(Position::parser().parse_to_end("top right bottom").is_err());
    assert!(Position::parser().parse_to_end("20").is_err());
  }
}
