use super::*;

#[test]
fn as_number_returns_some_for_percentage() {
  let val = CssValue::percentage(75.0);
  assert_eq!(val.as_number(), Some(75.0));
}

#[test]
fn as_number_returns_some_for_dimension() {
  let val = CssValue::dimension(10.0, "px");
  assert_eq!(val.as_number(), Some(10.0));
}

#[test]
fn as_number_returns_none_for_none_variant() {
  assert_eq!(CssValue::None.as_number(), None);
}

#[test]
fn as_percentage_returns_none_for_number() {
  let val = CssValue::number(42.0);
  assert_eq!(val.as_percentage(), None);
}

#[test]
fn as_string_returns_none_for_number() {
  let val = CssValue::number(42.0);
  assert!(val.as_string().is_none());
}

#[test]
fn as_angle_returns_some_for_angle_variant() {
  use crate::css_types::Angle;
  let angle = Angle::new(45.0, "deg");
  let val = CssValue::Angle(angle.clone());
  let result = val.as_angle();
  assert!(
    result.is_some(),
    "as_angle() should return Some for CssValue::Angle"
  );
  assert_eq!(result.unwrap().value, angle.value);
}

#[test]
fn as_angle_returns_none_for_ident() {
  assert!(CssValue::ident("auto").as_angle().is_none());
}

#[test]
fn as_color_returns_some_for_color_variant() {
  use crate::css_types::color::{Color, NamedColor};
  let color = Color::Named(NamedColor::new("blue".to_string()));
  let val = CssValue::Color(color);
  let result = val.as_color();
  assert!(
    result.is_some(),
    "as_color() should return Some for CssValue::Color"
  );
}

#[test]
fn as_dimension_returns_none_for_percentage() {
  assert!(CssValue::percentage(50.0).as_dimension().is_none());
}

#[test]
fn is_angle_true_for_angle_variant() {
  use crate::css_types::Angle;
  let angle: CssValue = Angle::new(45.0, "deg").into();
  assert!(angle.is_angle());
}

#[test]
fn is_color_true_for_color_variant() {
  use crate::css_types::color::{Color, NamedColor};
  let color: CssValue = Color::Named(NamedColor::new("blue".to_string())).into();
  assert!(color.is_color());
}

#[test]
fn has_unit_returns_false_for_number() {
  assert!(!CssValue::number(42.0).has_unit("px"));
}

#[test]
fn display_string_variant_wraps_in_quotes() {
  let val = CssValue::String("hello".to_string());
  assert_eq!(val.to_string(), "\"hello\"");
}

#[test]
fn display_angle_variant() {
  use crate::css_types::Angle;
  let val: CssValue = Angle::new(90.0, "deg").into();
  assert_eq!(val.to_string(), "90deg");
}

#[test]
fn display_color_variant() {
  use crate::css_types::color::{Color, NamedColor};
  let val: CssValue = Color::Named(NamedColor::new("red".to_string())).into();
  assert_eq!(val.to_string(), "red");
}

#[test]
fn display_length_variant() {
  use crate::css_types::Length;
  let val: CssValue = Length::new(10.0, "px".to_string()).into();
  assert_eq!(val.to_string(), "10px");
}

#[test]
fn display_token_variant_uses_debug_format() {
  use crate::token_types::SimpleToken;
  let val = CssValue::Token(SimpleToken::Comma);
  let formatted = val.to_string();
  // The Debug format of SimpleToken::Comma is "Comma"
  assert!(
    formatted.contains("Comma"),
    "Token variant display should use Debug format; got: {formatted}"
  );
}

#[test]
fn display_none_variant() {
  assert_eq!(CssValue::None.to_string(), "none");
}

#[test]
fn from_simple_token_string_becomes_css_value_string() {
  use crate::token_types::SimpleToken;
  let token = SimpleToken::String("Arial".to_string());
  let val: CssValue = token.into();
  assert!(
    val.is_string(),
    "SimpleToken::String should convert to CssValue::String"
  );
  assert_eq!(val.as_string(), Some(&"Arial".to_string()));
}

#[test]
fn from_simple_token_dimension_becomes_css_value_dimension() {
  use crate::token_types::SimpleToken;
  let token = SimpleToken::Dimension {
    value: 10.0,
    unit: "px".to_string(),
  };
  let val: CssValue = token.into();
  assert!(val.is_dimension());
  assert_eq!(val.as_dimension(), Some((10.0, &"px".to_string())));
}

#[test]
fn from_simple_token_percentage_becomes_css_value_percentage() {
  use crate::token_types::SimpleToken;
  let token = SimpleToken::Percentage(0.5);
  let val: CssValue = token.into();
  assert!(val.is_percentage());
  assert_eq!(val.as_percentage(), Some(0.5));
}

// A writer that fails after N writes drives each `write!(f, ...)?` in the Function and
// Sequence Display arms to its `Err` branch.

use std::fmt::{self, Write};

/// A writer that succeeds `remaining` times, then fails every subsequent write.
struct FailAfter {
  remaining: usize,
}

impl Write for FailAfter {
  fn write_str(&mut self, _s: &str) -> fmt::Result {
    if self.remaining == 0 {
      return Err(fmt::Error);
    }
    self.remaining -= 1;
    Ok(())
  }
}

#[test]
fn display_sequence_propagates_writer_errors_on_every_inner_write() {
  // Two items so the `i > 0` separator branch is exercised too.
  let value = CssValue::Sequence(vec![CssValue::Number(1.0), CssValue::Number(2.0)]);

  let mut saw_err = false;
  let mut saw_ok = false;
  for remaining in 0..32 {
    let mut writer = FailAfter { remaining };
    match write!(writer, "{}", value) {
      Ok(()) => saw_ok = true,
      Err(_) => saw_err = true,
    }
  }
  assert!(saw_err, "a failing writer must surface a formatting error");
  assert!(saw_ok, "a permissive writer must complete formatting");
}
