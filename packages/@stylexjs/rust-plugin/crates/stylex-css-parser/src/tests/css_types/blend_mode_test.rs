/*!
Blend mode CSS type tests.


Test CSS Type: <blend-mode>
Tests parsing of all 16 standard blend mode keywords.
*/

use crate::css_types::blend_mode::BlendMode;

#[cfg(test)]
mod test_css_type_blend_mode {
  use super::*;

  #[test]
  fn parses_valid_blend_mode_values() {
    let result = BlendMode::parser().parse_to_end("normal").unwrap();
    assert_eq!(result.to_string(), "normal");

    let result = BlendMode::parser().parse_to_end("multiply").unwrap();
    assert_eq!(result.to_string(), "multiply");

    let result = BlendMode::parser().parse_to_end("screen").unwrap();
    assert_eq!(result.to_string(), "screen");

    let result = BlendMode::parser().parse_to_end("overlay").unwrap();
    assert_eq!(result.to_string(), "overlay");

    let result = BlendMode::parser().parse_to_end("darken").unwrap();
    assert_eq!(result.to_string(), "darken");

    let result = BlendMode::parser().parse_to_end("lighten").unwrap();
    assert_eq!(result.to_string(), "lighten");

    let result = BlendMode::parser().parse_to_end("color-dodge").unwrap();
    assert_eq!(result.to_string(), "color-dodge");

    let result = BlendMode::parser().parse_to_end("color-burn").unwrap();
    assert_eq!(result.to_string(), "color-burn");

    let result = BlendMode::parser().parse_to_end("hard-light").unwrap();
    assert_eq!(result.to_string(), "hard-light");

    let result = BlendMode::parser().parse_to_end("soft-light").unwrap();
    assert_eq!(result.to_string(), "soft-light");

    let result = BlendMode::parser().parse_to_end("difference").unwrap();
    assert_eq!(result.to_string(), "difference");

    let result = BlendMode::parser().parse_to_end("exclusion").unwrap();
    assert_eq!(result.to_string(), "exclusion");

    let result = BlendMode::parser().parse_to_end("hue").unwrap();
    assert_eq!(result.to_string(), "hue");

    let result = BlendMode::parser().parse_to_end("saturation").unwrap();
    assert_eq!(result.to_string(), "saturation");

    let result = BlendMode::parser().parse_to_end("color").unwrap();
    assert_eq!(result.to_string(), "color");

    let result = BlendMode::parser().parse_to_end("luminosity").unwrap();
    assert_eq!(result.to_string(), "luminosity");
  }

  #[test]
  fn rejects_invalid_blend_mode_values() {
    assert!(BlendMode::parser().parse_to_end("invalid").is_err());
    assert!(BlendMode::parser().parse_to_end("blend").is_err());
    assert!(BlendMode::parser().parse_to_end("123").is_err());
  }
}
