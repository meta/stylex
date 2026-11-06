use super::*;

#[test]
fn test_blend_mode_from_str() {
  assert_eq!(BlendMode::parse("normal"), Some(BlendMode::Normal));
  assert_eq!(BlendMode::parse("multiply"), Some(BlendMode::Multiply));
  assert_eq!(BlendMode::parse("screen"), Some(BlendMode::Screen));
  assert_eq!(BlendMode::parse("overlay"), Some(BlendMode::Overlay));
  assert_eq!(BlendMode::parse("darken"), Some(BlendMode::Darken));
  assert_eq!(BlendMode::parse("lighten"), Some(BlendMode::Lighten));
  assert_eq!(BlendMode::parse("color-dodge"), Some(BlendMode::ColorDodge));
  assert_eq!(BlendMode::parse("color-burn"), Some(BlendMode::ColorBurn));
  assert_eq!(BlendMode::parse("hard-light"), Some(BlendMode::HardLight));
  assert_eq!(BlendMode::parse("soft-light"), Some(BlendMode::SoftLight));
  assert_eq!(BlendMode::parse("difference"), Some(BlendMode::Difference));
  assert_eq!(BlendMode::parse("exclusion"), Some(BlendMode::Exclusion));
  assert_eq!(BlendMode::parse("hue"), Some(BlendMode::Hue));
  assert_eq!(BlendMode::parse("saturation"), Some(BlendMode::Saturation));
  assert_eq!(BlendMode::parse("color"), Some(BlendMode::Color));
  assert_eq!(BlendMode::parse("luminosity"), Some(BlendMode::Luminosity));

  assert_eq!(BlendMode::parse("invalid"), None);
  assert_eq!(BlendMode::parse("NORMAL"), None);
  assert_eq!(BlendMode::parse(""), None);
}
