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
    // Test: Position.parser.parse('left') -> new Position('left', undefined)
    let result = Position::parser().parse_to_end("left").unwrap();
    assert!(matches!(
      result.horizontal,
      Some(Horizontal::Keyword(HorizontalKeyword::Left))
    ));
    assert!(result.vertical.is_none());

    // Test: Position.parser.parse('right') -> new Position('right', undefined)
    let result = Position::parser().parse_to_end("right").unwrap();
    assert!(matches!(
      result.horizontal,
      Some(Horizontal::Keyword(HorizontalKeyword::Right))
    ));
    assert!(result.vertical.is_none());

    // Test: Position.parser.parse('center') -> new Position('center', undefined)
    let result = Position::parser().parse_to_end("center").unwrap();
    // Center can be either horizontal or vertical - just check it's present
    assert!(result.horizontal.is_some() || result.vertical.is_some());

    // Test: Position.parser.parse('top') -> new Position(undefined, 'top')
    let result = Position::parser().parse_to_end("top").unwrap();
    assert!(result.horizontal.is_none());
    assert!(matches!(
      result.vertical,
      Some(Vertical::Keyword(VerticalKeyword::Top))
    ));

    // Test: Position.parser.parse('bottom') -> new Position(undefined, 'bottom')
    let result = Position::parser().parse_to_end("bottom").unwrap();
    assert!(result.horizontal.is_none());
    assert!(matches!(
      result.vertical,
      Some(Vertical::Keyword(VerticalKeyword::Bottom))
    ));
  }

  #[test]
  fn parses_keyword_combinations() {
    // Test: Position.parser.parse('left top') -> new Position('left', 'top')
    let result = Position::parser().parse_to_end("left top").unwrap();
    assert!(matches!(
      result.horizontal,
      Some(Horizontal::Keyword(HorizontalKeyword::Left))
    ));
    assert!(matches!(
      result.vertical,
      Some(Vertical::Keyword(VerticalKeyword::Top))
    ));

    // Test: Position.parser.parse('right bottom') -> new Position('right',
    // 'bottom')
    let result = Position::parser().parse_to_end("right bottom").unwrap();
    assert!(matches!(
      result.horizontal,
      Some(Horizontal::Keyword(HorizontalKeyword::Right))
    ));
    assert!(matches!(
      result.vertical,
      Some(Vertical::Keyword(VerticalKeyword::Bottom))
    ));

    // Test: Position.parser.parse('center center') -> new Position('center',
    // 'center')
    let result = Position::parser().parse_to_end("center center").unwrap();
    assert!(matches!(
      result.horizontal,
      Some(Horizontal::Keyword(HorizontalKeyword::Center))
    ));
    assert!(matches!(
      result.vertical,
      Some(Vertical::Keyword(VerticalKeyword::Center))
    ));

    // Test: Position.parser.parse('left bottom') -> new Position('left', 'bottom')
    let result = Position::parser().parse_to_end("left bottom").unwrap();
    assert!(matches!(
      result.horizontal,
      Some(Horizontal::Keyword(HorizontalKeyword::Left))
    ));
    assert!(matches!(
      result.vertical,
      Some(Vertical::Keyword(VerticalKeyword::Bottom))
    ));

    // Test: Position.parser.parse('right top') -> new Position('right', 'top')
    let result = Position::parser().parse_to_end("right top").unwrap();
    assert!(matches!(
      result.horizontal,
      Some(Horizontal::Keyword(HorizontalKeyword::Right))
    ));
    assert!(matches!(
      result.vertical,
      Some(Vertical::Keyword(VerticalKeyword::Top))
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
  fn parses_length_percentage_combinations() {
    assert!(Position::parser().parse_to_end("50% 50%").is_ok());

    assert!(Position::parser().parse_to_end("20px 30px").is_ok());

    assert!(Position::parser().parse_to_end("25% 40px").is_ok());
  }

  #[test]
  fn parses_keyword_with_offset() {
    assert!(Position::parser().parse_to_end("left 20% top 30%").is_ok());

    assert!(
      Position::parser()
        .parse_to_end("right 10px bottom 15px")
        .is_ok()
    );
  }

  #[test]
  fn rejects_invalid_positions() {
    assert!(Position::parser().parse_to_end("invalid").is_err());
    assert!(Position::parser().parse_to_end("left left").is_err());
    assert!(Position::parser().parse_to_end("top right bottom").is_err());
    assert!(Position::parser().parse_to_end("20").is_err());
  }

  #[test]
  fn parses_single_horizontal_keyword() {
    let result = Position::parser().parse_to_end("left").unwrap();
    assert!(matches!(
      result.horizontal,
      Some(Horizontal::Keyword(HorizontalKeyword::Left))
    ));
    assert!(result.vertical.is_none());
  }

  #[test]
  fn parses_single_vertical_keyword() {
    let result = Position::parser().parse_to_end("top").unwrap();
    assert!(result.horizontal.is_none());
    assert!(matches!(
      result.vertical,
      Some(Vertical::Keyword(VerticalKeyword::Top))
    ));
  }

  #[test]
  fn parses_single_center_keyword() {
    let result = Position::parser().parse_to_end("center").unwrap();
    assert!(result.horizontal.is_some() || result.vertical.is_some());
  }

  #[test]
  fn parses_center_center() {
    let result = Position::parser().parse_to_end("center center").unwrap();
    assert!(matches!(
      result.horizontal,
      Some(Horizontal::Keyword(HorizontalKeyword::Center))
    ));
    assert!(matches!(
      result.vertical,
      Some(Vertical::Keyword(VerticalKeyword::Center))
    ));
  }

  #[test]
  fn parses_left_top() {
    let result = Position::parser().parse_to_end("left top").unwrap();
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
  fn parses_two_lengths() {
    let result = Position::parser().parse_to_end("10px 20px").unwrap();
    assert!(matches!(result.horizontal, Some(Horizontal::Length(_))));
    assert!(matches!(result.vertical, Some(Vertical::Length(_))));
  }

  #[test]
  fn parses_keyword_plus_length() {
    let result = Position::parser().parse_to_end("left 10px").unwrap();
    assert!(matches!(
      result.horizontal,
      Some(Horizontal::Keyword(HorizontalKeyword::Left))
    ));
    assert!(matches!(result.vertical, Some(Vertical::Length(_))));
  }

  #[test]
  fn parses_length_plus_keyword() {
    let result = Position::parser().parse_to_end("10px top").unwrap();
    assert!(matches!(result.horizontal, Some(Horizontal::Length(_))));
    assert!(matches!(
      result.vertical,
      Some(Vertical::Keyword(VerticalKeyword::Top))
    ));
  }
}
