use super::*;

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
fn test_two_lengths() {
  let result = Position::parser().parse("50%");
  if let Ok(pos) = result {
    assert!(matches!(pos.horizontal, Some(Horizontal::Length(_))));
    assert!(matches!(pos.vertical, Some(Vertical::Length(_))));
  }
}
