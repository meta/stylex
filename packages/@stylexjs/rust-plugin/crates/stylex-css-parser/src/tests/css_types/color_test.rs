/*!
Color CSS type tests.
*/

use crate::css_types::color::Color;

#[cfg(test)]
mod test_css_type_color {
  use stylex_macros::stylex_panic;

  use super::*;

  #[test]
  fn parses_space_separated_rgba_values() {
    let color1 = Color::parse().parse_to_end("rgba(255 0 0 / 0.5)").unwrap();
    match color1 {
      Color::Rgba(ref rgba) => {
        assert_eq!(rgba.r, 255.0);
        assert_eq!(rgba.g, 0.0);
        assert_eq!(rgba.b, 0.0);
        assert_eq!(rgba.a, 0.5);
      },
      _ => stylex_panic!("Expected Rgba"),
    }

    let color2 = Color::parse().parse_to_end("rgba(0 255 0 / 0.8)").unwrap();
    match color2 {
      Color::Rgba(ref rgba) => {
        assert_eq!(rgba.r, 0.0);
        assert_eq!(rgba.g, 255.0);
        assert_eq!(rgba.b, 0.0);
        assert_eq!(rgba.a, 0.8);
      },
      _ => stylex_panic!("Expected Rgba"),
    }

    let color3 = Color::parse().parse_to_end("rgba(0 0 255 / 50%)").unwrap();
    match color3 {
      Color::Rgba(ref rgba) => {
        assert_eq!(rgba.r, 0.0);
        assert_eq!(rgba.g, 0.0);
        assert_eq!(rgba.b, 255.0);
        assert_eq!(rgba.a, 0.5);
      },
      _ => stylex_panic!("Expected Rgba"),
    }
  }

  #[test]
  fn parses_modern_color_spaces_with_none_values() {
    let oklch_none = Color::parse()
      .parse_to_end("oklch(none 0.15 none)")
      .unwrap();
    match oklch_none {
      Color::Oklch(ref oklch) => {
        assert_eq!(oklch.l, 0.0); // 'none' maps to 0
        assert_eq!(oklch.c, 0.15);
        assert_eq!(oklch.h.value, 0.0); // 'none' * 360 = 0
      },
      _ => stylex_panic!("Expected Oklch"),
    }

    let oklab_none = Color::parse().parse_to_end("oklab(0.5 none none)").unwrap();
    match oklab_none {
      Color::Oklab(ref oklab) => {
        assert_eq!(oklab.l, 0.5);
        assert_eq!(oklab.a, 0.0); // 'none' maps to 0
        assert_eq!(oklab.b, 0.0); // 'none' maps to 0
      },
      _ => stylex_panic!("Expected Oklab"),
    }
  }

  #[test]
  fn comprehensive_color_parsing() {
    let test_cases = vec![
      "red",
      "blue",
      "green",
      "black",
      "white",
      "transparent",
      "currentColor",
      "#ff0000",
      "#00ff00",
      "#0000ff",
      "#abc",
      "#def",
      "#123456",
      "#abcdef",
      "rgb(255, 0, 0)",
      "rgb(0, 255, 0)",
      "rgb(0, 0, 255)",
      "rgb(128, 128, 128)",
      "rgba(255, 0, 0, 0.5)",
      "rgba(0, 255, 0, 1.0)",
      "rgba(0, 0, 255, 0.25)",
      "hsl(0, 100%, 50%)",
      "hsl(120, 100%, 50%)",
      "hsl(240, 100%, 50%)",
      "hsla(0, 100%, 50%, 0.5)",
      "hsla(120, 100%, 50%, 1.0)",
      "lch(50% 30 180)",
      "oklch(0.7 0.15 180)",
      "oklab(0.7 -0.15 0.1)",
    ];

    for input in test_cases {
      let color = Color::parse().parse_to_end(input).unwrap();

      assert!(
        !color.to_string().is_empty(),
        "Should produce output for: {}",
        input
      );

      let reparsed = Color::parse().parse_to_end(&color.to_string());
      assert!(
        reparsed.is_ok(),
        "Should reparse successfully: {} -> {}",
        input,
        color
      );
    }
  }

  #[test]
  fn whitespace_handling() {
    let test_cases = vec![
      "rgb(255,0,0)",            // No spaces
      "rgb( 255 , 0 , 0 )",      // Extra spaces
      "hsl( 120 , 100% , 50% )", // HSL with spaces
      "rgba(255, 0, 0, 0.5)",    // RGBA normal
    ];

    for input in test_cases {
      let color = Color::parse().parse_to_end(input).unwrap();
      assert!(!color.to_string().is_empty(), "Should parse: {}", input);
    }
  }

  #[test]
  fn rejects_invalid_colors() {
    assert!(Color::parse().parse_to_end("invalid").is_err());
    assert!(Color::parse().parse_to_end("#gggggg").is_err());
    assert!(Color::parse().parse_to_end("rgb(256, 0, 0)").is_err());
  }

  #[test]
  fn parses_space_separated_rgb_additional() {
    let color = Color::parse().parse_to_end("rgb(255 0 0)").unwrap();
    match color {
      Color::Rgb(ref rgb) => {
        assert_eq!(rgb.r, 255.0);
        assert_eq!(rgb.g, 0.0);
        assert_eq!(rgb.b, 0.0);
      },
      _ => stylex_panic!("Expected Rgb"),
    }

    let color2 = Color::parse().parse_to_end("rgb(128 64 32)").unwrap();
    match color2 {
      Color::Rgb(ref rgb) => {
        assert_eq!(rgb.r, 128.0);
        assert_eq!(rgb.g, 64.0);
        assert_eq!(rgb.b, 32.0);
      },
      _ => stylex_panic!("Expected Rgb"),
    }
  }
}

#[cfg(test)]
mod the_optional_alpha_rewind {
  use super::*;

  /// The alpha reader rewinds the whitespace it skipped, and the closing-paren check does not
  /// skip it.
  #[test]
  fn a_space_before_the_closing_paren_is_refused_after_a_rewind() {
    for input in [
      "lch(50 100 180 )",
      "oklch(0.7 0.1 200deg )",
      "oklab(0.5 0.1 0.1 )",
    ] {
      assert!(
        Color::parse().parse_to_end(input).is_err(),
        "{input:?} should be refused"
      );
    }
  }

  #[test]
  fn the_same_space_is_accepted_when_an_alpha_follows_it() {
    match Color::parse().parse_to_end("lch(50 100 180 / 0.5)") {
      // An `lch` hue with no unit stays a bare number; `oklch` reads it as degrees.
      Ok(color) => assert_eq!(color.to_string(), "lch(50 100 180 / 0.5)"),
      Err(error) => panic!("expected the alpha form to parse: {error:?}"),
    }
  }
}
