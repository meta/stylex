use super::*;
use crate::css_types::Color;

// ── is_inset_ident ─────────────────────────────

#[test]
fn is_inset_ident_returns_true_for_inset() {
  assert!(is_inset_ident(&SimpleToken::Ident("inset".to_string())));
}

#[test]
fn is_inset_ident_returns_false_for_other_ident() {
  assert!(!is_inset_ident(&SimpleToken::Ident("none".to_string())));
}

#[test]
fn is_inset_ident_returns_false_for_non_ident() {
  // Exercises `else { false }` arm.
  assert!(!is_inset_ident(&SimpleToken::Whitespace));
  assert!(!is_inset_ident(&SimpleToken::Comma));
}

// ── is_none_ident ──────────────────────────────

#[test]
fn is_none_ident_returns_true_for_none() {
  assert!(is_none_ident(&SimpleToken::Ident("none".to_string())));
}

#[test]
fn is_none_ident_returns_false_for_other_ident() {
  assert!(!is_none_ident(&SimpleToken::Ident("inset".to_string())));
}

#[test]
fn is_none_ident_returns_false_for_non_ident() {
  // Exercises `else { false }` arm.
  assert!(!is_none_ident(&SimpleToken::Whitespace));
  assert!(!is_none_ident(&SimpleToken::Comma));
}

// ── BoxShadow::simple ─────────────────────────────────────────────────────

#[test]
fn simple_with_explicit_blur_and_spread() {
  use crate::css_types::Length;

  let color = Color::parse().parse_to_end("red").unwrap();
  let shadow = BoxShadow::simple(
    Length::new(5.0, "px"),
    Length::new(10.0, "px"),
    Some(Length::new(3.0, "px")),
    Some(Length::new(2.0, "px")),
    color,
    false,
  );
  assert_eq!(shadow.offset_x.value, 5.0);
  assert_eq!(shadow.offset_y.value, 10.0);
  assert_eq!(shadow.blur_radius.value, 3.0);
  assert_eq!(shadow.spread_radius.value, 2.0);
  assert!(!shadow.inset);
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
