// Tests extracted for properties/box_shadow.rs behaviors and edge cases.
// Source: crates/stylex-css-parser/src/properties/box_shadow.rs

use super::*;
use crate::css_types::{HashColor, NamedColor};

#[test]
fn test_box_shadow_creation() {
  let offset_x = Length::new(2.0, "px".to_string());
  let offset_y = Length::new(4.0, "px".to_string());
  let blur = Length::new(6.0, "px".to_string());
  let spread = Length::new(0.0, "px".to_string());
  let color = Color::Named(NamedColor::new("red".to_string()));

  let shadow = BoxShadow::new(
    offset_x.clone(),
    offset_y.clone(),
    blur.clone(),
    spread.clone(),
    color.clone(),
    false,
  );

  assert_eq!(shadow.offset_x, offset_x);
  assert_eq!(shadow.offset_y, offset_y);
  assert_eq!(shadow.blur_radius, blur);
  assert_eq!(shadow.spread_radius, spread);
  assert_eq!(shadow.color, color);
  assert!(!shadow.inset);
}

#[test]
fn test_box_shadow_simple_constructor() {
  let offset_x = Length::new(1.0, "px".to_string());
  let offset_y = Length::new(2.0, "px".to_string());
  let color = Color::Named(NamedColor::new("black".to_string()));

  let shadow = BoxShadow::simple(
    offset_x.clone(),
    offset_y.clone(),
    None,
    None,
    color.clone(),
    false,
  );

  assert_eq!(shadow.offset_x, offset_x);
  assert_eq!(shadow.offset_y, offset_y);
  assert_eq!(shadow.blur_radius.value, 0.0);
  assert_eq!(shadow.spread_radius.value, 0.0);
  assert_eq!(shadow.color, color);
  assert!(!shadow.inset);
}

#[test]
fn test_box_shadow_inset() {
  let offset_x = Length::new(1.0, "px".to_string());
  let offset_y = Length::new(1.0, "px".to_string());
  let blur = Length::new(3.0, "px".to_string());
  let spread = Length::new(0.0, "px".to_string());
  let color = Color::Hash(HashColor::new("#000000".to_string()));

  let inset_shadow = BoxShadow::new(offset_x, offset_y, blur, spread, color, true);

  assert!(inset_shadow.inset);
}

#[test]
fn test_box_shadow_display() {
  let offset_x = Length::new(2.0, "px".to_string());
  let offset_y = Length::new(4.0, "px".to_string());
  let blur = Length::new(6.0, "px".to_string());
  let spread = Length::new(2.0, "px".to_string());
  let color = Color::Named(NamedColor::new("red".to_string()));

  let shadow = BoxShadow::new(offset_x, offset_y, blur, spread, color, false);
  assert_eq!(shadow.to_string(), "2px 4px 6px 2px red");

  let inset_shadow = BoxShadow::new(
    Length::new(1.0, "px".to_string()),
    Length::new(1.0, "px".to_string()),
    Length::new(2.0, "px".to_string()),
    Length::new(0.0, "px".to_string()),
    Color::Named(NamedColor::new("blue".to_string())),
    true,
  );
  assert_eq!(inset_shadow.to_string(), "inset 1px 1px 2px blue");
}

#[test]
fn test_box_shadow_display_zero_values() {
  let shadow = BoxShadow::new(
    Length::new(1.0, "px".to_string()),
    Length::new(2.0, "px".to_string()),
    Length::new(0.0, "px".to_string()), // Zero blur
    Length::new(0.0, "px".to_string()), // Zero spread
    Color::Named(NamedColor::new("black".to_string())),
    false,
  );

  // Should omit zero blur and spread values
  assert_eq!(shadow.to_string(), "1px 2px black");
}

#[test]
fn test_box_shadow_list_creation() {
  let shadow1 = BoxShadow::simple(
    Length::new(1.0, "px".to_string()),
    Length::new(1.0, "px".to_string()),
    None,
    None,
    Color::Named(NamedColor::new("red".to_string())),
    false,
  );

  let shadow2 = BoxShadow::simple(
    Length::new(2.0, "px".to_string()),
    Length::new(2.0, "px".to_string()),
    Some(Length::new(4.0, "px".to_string())),
    None,
    Color::Named(NamedColor::new("blue".to_string())),
    true,
  );

  let shadow_list = BoxShadowList::new(vec![shadow1, shadow2]);
  assert_eq!(shadow_list.shadows.len(), 2);
}

#[test]
fn test_box_shadow_list_display() {
  let shadow1 = BoxShadow::new(
    Length::new(1.0, "px".to_string()),
    Length::new(1.0, "px".to_string()),
    Length::new(0.0, "px".to_string()),
    Length::new(0.0, "px".to_string()),
    Color::Named(NamedColor::new("red".to_string())),
    false,
  );

  let shadow2 = BoxShadow::new(
    Length::new(2.0, "px".to_string()),
    Length::new(2.0, "px".to_string()),
    Length::new(4.0, "px".to_string()),
    Length::new(0.0, "px".to_string()),
    Color::Named(NamedColor::new("blue".to_string())),
    true,
  );

  let shadow_list = BoxShadowList::new(vec![shadow1, shadow2]);
  assert_eq!(
    shadow_list.to_string(),
    "1px 1px red, inset 2px 2px 4px blue"
  );
}

#[test]
fn test_box_shadow_parser_creation() {
  // Basic test that parsers can be created
  let _shadow_parser = BoxShadow::parser();
  let _list_parser = BoxShadowList::parser();
}

#[test]
fn test_box_shadow_equality() {
  let shadow1 = BoxShadow::simple(
    Length::new(1.0, "px".to_string()),
    Length::new(1.0, "px".to_string()),
    None,
    None,
    Color::Named(NamedColor::new("red".to_string())),
    false,
  );

  let shadow2 = BoxShadow::simple(
    Length::new(1.0, "px".to_string()),
    Length::new(1.0, "px".to_string()),
    None,
    None,
    Color::Named(NamedColor::new("red".to_string())),
    false,
  );

  let shadow3 = BoxShadow::simple(
    Length::new(2.0, "px".to_string()),
    Length::new(2.0, "px".to_string()),
    None,
    None,
    Color::Named(NamedColor::new("red".to_string())),
    false,
  );

  assert_eq!(shadow1, shadow2);
  assert_ne!(shadow1, shadow3);
}

#[test]
fn test_box_shadow_common_values() {
  // Test common box-shadow patterns

  // Simple drop shadow
  let drop_shadow = BoxShadow::simple(
    Length::new(0.0, "px".to_string()),
    Length::new(2.0, "px".to_string()),
    Some(Length::new(4.0, "px".to_string())),
    None,
    Color::Hash(HashColor::new("#00000026".to_string())), // 15% opacity black
    false,
  );
  assert!(!drop_shadow.inset);

  // Inner shadow
  let inner_shadow = BoxShadow::simple(
    Length::new(0.0, "px".to_string()),
    Length::new(1.0, "px".to_string()),
    Some(Length::new(2.0, "px".to_string())),
    None,
    Color::Hash(HashColor::new("#0000001a".to_string())), // 10% opacity black
    true,
  );
  assert!(inner_shadow.inset);

  // No shadow (all zero)
  let no_shadow = BoxShadow::simple(
    Length::new(0.0, "px".to_string()),
    Length::new(0.0, "px".to_string()),
    None,
    None,
    Color::Named(NamedColor::new("transparent".to_string())),
    false,
  );
  assert_eq!(no_shadow.offset_x.value, 0.0);
  assert_eq!(no_shadow.offset_y.value, 0.0);
}

#[test]
fn test_box_shadow_with_spread() {
  let shadow_with_spread = BoxShadow::new(
    Length::new(0.0, "px".to_string()),
    Length::new(0.0, "px".to_string()),
    Length::new(10.0, "px".to_string()),
    Length::new(5.0, "px".to_string()), // Positive spread
    Color::Named(NamedColor::new("black".to_string())),
    false,
  );

  assert_eq!(shadow_with_spread.spread_radius.value, 5.0);
  assert_eq!(shadow_with_spread.to_string(), "0px 0px 10px 5px black");
}
