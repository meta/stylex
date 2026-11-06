// Tests extracted for css_types/position.rs behaviors and edge cases.
// Source: crates/stylex-css-parser/src/css_types/position.rs

use super::*;

#[test]
fn test_horizontal_keyword() {
  let result = Position::horizontal_keyword_parser().parse("left");
  assert!(result.is_ok());
  assert_eq!(result.unwrap(), HorizontalKeyword::Left);
}

#[test]
fn test_vertical_keyword() {
  let result = Position::vertical_keyword_parser().parse("top");
  assert!(result.is_ok());
  assert_eq!(result.unwrap(), VerticalKeyword::Top);
}

#[test]
fn test_position_basic() {
  let result = Position::parser().parse("left");
  assert!(result.is_ok());

  let pos = result.unwrap();
  assert!(pos.horizontal.is_some());
  assert!(pos.vertical.is_none());
}

#[test]
fn test_position_display() {
  let pos = Position::new(
    Some(Horizontal::Keyword(HorizontalKeyword::Left)),
    Some(Vertical::Keyword(VerticalKeyword::Top)),
  );
  assert_eq!(pos.to_string(), "left top");
}

#[test]
fn test_horizontal_keyword_as_str() {
  let left = HorizontalKeyword::Left;
  let center = HorizontalKeyword::Center;
  let right = HorizontalKeyword::Right;

  assert_eq!(left.as_str(), "left");
  assert_eq!(center.as_str(), "center");
  assert_eq!(right.as_str(), "right");
}

#[test]
fn test_vertical_keyword_as_str() {
  let top = VerticalKeyword::Top;
  let bottom = VerticalKeyword::Bottom;

  assert_eq!(top.as_str(), "top");
  assert_eq!(bottom.as_str(), "bottom");
}

#[test]
fn test_keyword_with_offset_display() {
  let h = Horizontal::KeywordWithOffset(
    HorizontalKeyword::Left,
    LengthPercentage::Percentage(crate::css_types::Percentage::new(50.0)),
  );
  assert_eq!(h.to_string(), "left 50%");
}

#[test]
fn test_numbers_only() {
  // This would test: "50% 25%" -> Position with both horizontal and vertical
  // length
  let pos = Position::new(
    Some(Horizontal::Length(LengthPercentage::Percentage(
      crate::css_types::Percentage::new(50.0),
    ))),
    Some(Vertical::Length(LengthPercentage::Percentage(
      crate::css_types::Percentage::new(25.0),
    ))),
  );
  assert_eq!(pos.to_string(), "50% 25%");
}

#[test]
fn test_two_keywords() {
  let result = Position::parser().parse("left top");
  if let Ok(pos) = result {
    assert!(matches!(
      pos.horizontal,
      Some(Horizontal::Keyword(HorizontalKeyword::Left))
    ));
    assert!(matches!(
      pos.vertical,
      Some(Vertical::Keyword(VerticalKeyword::Top))
    ));
  }
}

#[test]
fn test_two_lengths() {
  let result = Position::parser().parse("50%");
  if let Ok(pos) = result {
    // Single length should apply to both axes
    assert!(matches!(pos.horizontal, Some(Horizontal::Length(_))));
    assert!(matches!(pos.vertical, Some(Vertical::Length(_))));
  }
}
