// Tests extracted for css_types/color.rs behaviors and edge cases.
// Source: crates/stylex-css-parser/src/css_types/color.rs

use super::*;

#[test]
fn test_named_color_validation() {
  assert!(NamedColor::is_valid_named_color("red"));
  assert!(NamedColor::is_valid_named_color("blue"));
  assert!(NamedColor::is_valid_named_color("transparent"));
  assert!(NamedColor::is_valid_named_color("currentcolor"));

  // Case insensitive
  assert!(NamedColor::is_valid_named_color("RED"));
  assert!(NamedColor::is_valid_named_color("Blue"));

  // Invalid
  assert!(!NamedColor::is_valid_named_color("notacolor"));
  assert!(!NamedColor::is_valid_named_color(""));
}

#[test]
fn test_named_color_display() {
  let color = NamedColor::new("red".to_string());
  assert_eq!(color.to_string(), "red");
}

#[test]
fn test_hex_color_validation() {
  // Valid formats
  assert!(HashColor::is_valid_hex("F00")); // 3-digit
  assert!(HashColor::is_valid_hex("FF0000")); // 6-digit
  assert!(HashColor::is_valid_hex("FF0000FF")); // 8-digit with alpha
  assert!(HashColor::is_valid_hex("123abc")); // lowercase

  // Invalid formats
  assert!(!HashColor::is_valid_hex("GG0000")); // invalid hex
  assert!(!HashColor::is_valid_hex("FF00")); // wrong length
  assert!(!HashColor::is_valid_hex("")); // empty
}

#[test]
fn test_hex_color_rgb_extraction() {
  // 6-digit hex
  let color = HashColor::new("FF0000".to_string());
  assert_eq!(color.r(), 255);
  assert_eq!(color.g(), 0);
  assert_eq!(color.b(), 0);
  assert_eq!(color.a(), 1.0);

  // 3-digit hex
  let short_color = HashColor::new("F0A".to_string());
  assert_eq!(short_color.r(), 255); // F -> FF
  assert_eq!(short_color.g(), 0); // 0 -> 00
  assert_eq!(short_color.b(), 170); // A -> AA

  // 8-digit with alpha
  let alpha_color = HashColor::new("FF000080".to_string());
  assert!((alpha_color.a() - 0.5).abs() < 0.01); // 80 hex = 128 dec ≈ 0.5 alpha
}

#[test]
fn test_hex_color_display() {
  let color = HashColor::new("FF0000".to_string());
  assert_eq!(color.to_string(), "#FF0000");
}

#[test]
fn test_rgb_color_display() {
  let color = Rgb::new(255.0, 0.0, 0.0);
  assert_eq!(color.to_string(), "rgb(255,0,0)");
}

#[test]
fn test_rgba_color_display() {
  let color = Rgba::new(255.0, 0.0, 0.0, 0.5);
  assert_eq!(color.to_string(), "rgba(255,0,0,0.5)");
}

#[test]
fn test_hsl_color_display() {
  let color = Hsl::from_primitives(360.0, 100.0, 50.0);
  assert_eq!(color.to_string(), "hsl(360deg,100%,50%)");
}

#[test]
fn test_hsla_color_display() {
  let color = Hsla::from_primitives(360.0, 100.0, 50.0, 0.8);
  assert_eq!(color.to_string(), "hsla(360deg,100%,50%,0.8)");
}

#[test]
fn test_color_enum_display() {
  let named = Color::Named(NamedColor::new("red".to_string()));
  assert_eq!(named.to_string(), "red");

  let hash = Color::Hash(HashColor::new("FF0000".to_string()));
  assert_eq!(hash.to_string(), "#FF0000");

  let rgb = Color::Rgb(Rgb::new(255.0, 0.0, 0.0));
  assert_eq!(rgb.to_string(), "rgb(255,0,0)");
}

#[test]
fn test_color_parsers_creation() {
  // Basic test that parsers can be created
  let _named = NamedColor::parse();
  let _hash = HashColor::parse();
  let _rgb = Rgb::parse();
  let _rgba = Rgba::parse();
  let _hsl = Hsl::parse();
  let _hsla = Hsla::parse();
  let _lch = Lch::parse();
  let _oklch = Oklch::parse();
  let _oklab = Oklab::parse();
  let _color = Color::parse();
}

#[test]
fn test_lch_color_display() {
  let color = Lch::new_with_angle(50.0, 100.0, Angle::new(270.0, "deg".to_string()), None);
  assert_eq!(color.to_string(), "lch(50 100 270deg)");

  let color_with_alpha =
    Lch::new_with_angle(50.0, 100.0, Angle::new(270.0, "deg".to_string()), Some(0.8));
  assert_eq!(color_with_alpha.to_string(), "lch(50 100 270deg / 0.8)");
}

#[test]
fn test_oklch_color_display() {
  let color = Oklch::new(0.5, 0.1, Angle::new(270.0, "deg".to_string()), None);
  assert_eq!(color.to_string(), "oklch(0.5 0.1 270deg)");

  let color_with_alpha = Oklch::new(0.5, 0.1, Angle::new(270.0, "deg".to_string()), Some(0.8));
  assert_eq!(color_with_alpha.to_string(), "oklch(0.5 0.1 270deg / 0.8)");
}

#[test]
fn test_oklab_color_display() {
  let color = Oklab::new(0.5, 0.1, 0.1, None);
  assert_eq!(color.to_string(), "oklab(0.5 0.1 0.1)");

  let color_with_alpha = Oklab::new(0.5, 0.1, 0.1, Some(0.8));
  assert_eq!(color_with_alpha.to_string(), "oklab(0.5 0.1 0.1 / 0.8)");
}
