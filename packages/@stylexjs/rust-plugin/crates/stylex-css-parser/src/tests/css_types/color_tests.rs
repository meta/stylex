use super::*;

#[test]
fn test_hex_color_rgb_extraction() {
  let color = HashColor::new("FF0000".to_string());
  assert_eq!(color.r(), 255);
  assert_eq!(color.g(), 0);
  assert_eq!(color.b(), 0);
  assert_eq!(color.a(), 1.0);

  let short_color = HashColor::new("F0A".to_string());
  assert_eq!(short_color.r(), 255); // F -> FF
  assert_eq!(short_color.g(), 0); // 0 -> 00
  assert_eq!(short_color.b(), 170); // A -> AA

  let alpha_color = HashColor::new("FF000080".to_string());
  assert!((alpha_color.a() - 0.5).abs() < 0.01); // 80 hex = 128 dec ≈ 0.5 alpha
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
