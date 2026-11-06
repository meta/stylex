use super::*;
use crate::css_types::Color;

#[test]
fn is_inset_ident_returns_false_for_non_ident() {
  assert!(!is_inset_ident(&SimpleToken::Whitespace));
  assert!(!is_inset_ident(&SimpleToken::Comma));
}

#[test]
fn is_none_ident_returns_false_for_non_ident() {
  assert!(!is_none_ident(&SimpleToken::Whitespace));
  assert!(!is_none_ident(&SimpleToken::Comma));
}

#[test]
fn simple_with_none_blur_and_spread_defaults_to_zero() {
  use crate::css_types::Length;

  let color = Color::parse().parse_to_end("blue").unwrap();
  let shadow = BoxShadow::simple(
    Length::new(1.0, "px"),
    Length::new(2.0, "px"),
    None,
    None,
    color,
    true,
  );
  assert_eq!(shadow.blur_radius.value, 0.0);
  assert_eq!(shadow.blur_radius.unit, "px");
  assert_eq!(shadow.spread_radius.value, 0.0);
  assert_eq!(shadow.spread_radius.unit, "px");
  assert!(shadow.inset);
}
