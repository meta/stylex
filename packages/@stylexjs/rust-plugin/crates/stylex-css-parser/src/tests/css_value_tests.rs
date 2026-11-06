// Tests extracted for css_value.rs behaviors and edge cases.
// Source: crates/stylex-css-parser/src/css_value.rs

use super::*;

#[test]
fn test_css_value_creation() {
  let num = CssValue::number(42.0);
  let percent = CssValue::percentage(50.0);
  let dim = CssValue::dimension(10.0, "px");
  let str_val = CssValue::string("Arial");
  let ident = CssValue::ident("auto");

  assert_eq!(num.as_number(), Some(42.0));
  assert_eq!(percent.as_percentage(), Some(50.0));
  assert_eq!(dim.as_dimension(), Some((10.0, &"px".to_string())));
  assert_eq!(str_val.as_string(), Some(&"Arial".to_string()));
  assert_eq!(ident.as_string(), Some(&"auto".to_string()));
}

#[test]
fn test_type_checking() {
  let num = CssValue::number(42.0);
  let percent = CssValue::percentage(50.0);
  let dim = CssValue::dimension(10.0, "px");

  assert!(num.is_number());
  assert!(!num.is_percentage());
  assert!(!num.is_dimension());

  assert!(percent.is_percentage());
  assert!(!percent.is_number());

  assert!(dim.is_dimension());
  assert!(dim.has_unit("px"));
  assert!(!dim.has_unit("em"));
}

#[test]
fn test_function_value() {
  let func = CssValue::function(
    "rgb",
    vec![
      CssValue::number(255.0),
      CssValue::number(0.0),
      CssValue::number(0.0),
    ],
  );

  assert!(func.is_function());

  if let Some((name, args)) = func.as_function() {
    assert_eq!(name, "rgb");
    assert_eq!(args.len(), 3);
    assert_eq!(args[0].as_number(), Some(255.0));
  }
}

#[test]
fn test_sequence_value() {
  let seq = CssValue::sequence(vec![
    CssValue::number(1.0),
    CssValue::ident("solid"),
    CssValue::ident("red"),
  ]);

  assert!(seq.is_sequence());

  if let Some(items) = seq.as_sequence() {
    assert_eq!(items.len(), 3);
    assert!(items[0].is_number());
    assert!(items[1].is_ident());
    assert!(items[2].is_ident());
  }
}

#[test]
fn test_display_formatting() {
  let num = CssValue::number(42.0);
  let percent = CssValue::percentage(50.0);
  let dim = CssValue::dimension(10.0, "px");
  let func = CssValue::function(
    "calc",
    vec![
      CssValue::dimension(1.0, "px"),
      CssValue::ident("+"),
      CssValue::dimension(2.0, "em"),
    ],
  );

  assert_eq!(num.to_string(), "42");
  assert_eq!(percent.to_string(), "50%");
  assert_eq!(dim.to_string(), "10px");
  assert_eq!(func.to_string(), "calc(1px, +, 2em)");
}

#[test]
fn test_from_f64() {
  let val: CssValue = 42.0_f64.into();
  assert!(val.is_number());
  assert_eq!(val.as_number(), Some(42.0));
}

#[test]
fn test_from_string() {
  let val: CssValue = String::from("hello").into();
  assert!(val.is_string());
  assert_eq!(val.as_string(), Some(&"hello".to_string()));
}

#[test]
fn test_from_str() {
  let val: CssValue = "world".into();
  assert!(val.is_string());
}

#[test]
fn test_from_angle() {
  use crate::css_types::Angle;
  let angle = Angle::new(45.0, "deg".to_string());
  let val: CssValue = angle.into();
  assert!(val.is_angle());
}

#[test]
fn test_from_color() {
  use crate::css_types::color::{Color, NamedColor};
  let color = Color::Named(NamedColor::new("red".to_string()));
  let val: CssValue = color.into();
  assert!(val.is_color());
}

#[test]
fn test_from_length() {
  use crate::css_types::Length;
  let length = Length::new(10.0, "px".to_string());
  let val: CssValue = length.into();
  // CssValue::Length doesn't have is_length() - just check it's not none
  assert!(!val.is_none());
}

#[test]
fn test_from_percentage() {
  use crate::css_types::Percentage;
  let pct = Percentage::new(50.0);
  let val: CssValue = pct.into();
  assert!(val.is_percentage());
  assert_eq!(val.as_percentage(), Some(50.0));
}

#[test]
fn test_from_simple_token_number() {
  use crate::token_types::SimpleToken;
  let token = SimpleToken::Number(42.0);
  let val: CssValue = token.into();
  assert!(val.is_number());
}

#[test]
fn test_from_simple_token_ident() {
  use crate::token_types::SimpleToken;
  let token = SimpleToken::Ident("auto".to_string());
  let val: CssValue = token.into();
  assert!(val.is_ident());
}

#[test]
fn test_from_simple_token_unknown() {
  use crate::token_types::SimpleToken;
  // Tokens like Comma, Colon, etc. should become CssValue::Token
  let token = SimpleToken::Comma;
  let val: CssValue = token.into();
  matches!(val, CssValue::Token(_));
}

#[test]
fn test_is_none() {
  let none_val = CssValue::None;
  assert!(none_val.is_none());
  assert!(!none_val.is_number());
}

#[test]
fn test_get_unit() {
  let dim = CssValue::dimension(10.0, "px");
  assert_eq!(dim.get_unit(), Some(&"px".to_string()));

  let num = CssValue::number(42.0);
  assert_eq!(num.get_unit(), None);
}

#[test]
fn test_as_angle_returns_none_for_non_angle() {
  let num = CssValue::number(42.0);
  assert!(num.as_angle().is_none());
}

#[test]
fn test_as_color_returns_none_for_non_color() {
  let num = CssValue::number(42.0);
  assert!(num.as_color().is_none());
}

#[test]
fn test_as_function_returns_none_for_non_function() {
  let num = CssValue::number(42.0);
  assert!(num.as_function().is_none());
}

#[test]
fn test_as_sequence_returns_none_for_non_sequence() {
  let num = CssValue::number(42.0);
  assert!(num.as_sequence().is_none());
}
