/*!
Box shadow property tests.
*/

use crate::{
  css_types::color::*,
  properties::box_shadow::{BoxShadow, BoxShadowList},
};

#[cfg(test)]
mod test_css_property_box_shadow {
  use super::*;

  #[cfg(test)]
  mod box_shadow {
    use stylex_macros::stylex_panic;

    use super::*;

    #[test]
    fn valid_box_shadow_with_required_parameters_offsetx_offsety_color() {
      // Test: Valid: box-shadow with required parameters (offsetX, offsetY, color)

      let result = BoxShadow::parser()
        .parse_to_end("10px 5px #ff0000")
        .unwrap();

      assert_eq!(result.offset_x.value, 10.0);
      assert_eq!(result.offset_x.unit, "px");
      assert_eq!(result.offset_y.value, 5.0);
      assert_eq!(result.offset_y.unit, "px");
      assert_eq!(result.blur_radius.value, 0.0);
      assert_eq!(result.blur_radius.unit, "px");
      assert_eq!(result.spread_radius.value, 0.0);
      assert_eq!(result.spread_radius.unit, "px");

      if let Color::Hash(hash) = &result.color {
        assert_eq!(hash.value, "ff0000");
      } else {
        stylex_panic!("Expected Hash color");
      }

      assert!(!result.inset);
    }

    #[test]
    fn valid_box_shadow_with_blur_radius_offsetx_offsety_blurradius_color() {
      // Test: Valid: box-shadow with blur radius (offsetX, offsetY, blurRadius,
      // color)

      let result = BoxShadow::parser()
        .parse_to_end("10px 5px 15px #ff0000")
        .unwrap();

      assert_eq!(result.offset_x.value, 10.0);
      assert_eq!(result.offset_x.unit, "px");
      assert_eq!(result.offset_y.value, 5.0);
      assert_eq!(result.offset_y.unit, "px");
      assert_eq!(result.blur_radius.value, 15.0);
      assert_eq!(result.blur_radius.unit, "px");
      assert_eq!(result.spread_radius.value, 0.0);
      assert_eq!(result.spread_radius.unit, "px");

      if let Color::Hash(hash) = &result.color {
        assert_eq!(hash.value, "ff0000");
      } else {
        stylex_panic!("Expected Hash color");
      }

      assert!(!result.inset);
    }

    #[test]
    fn valid_box_shadow_with_blur_and_spread_radius_offsetx_offsety_blurradius_spreadradius_color()
    {
      // Test: Valid: box-shadow with blur and spread radius (offsetX, offsetY,
      // blurRadius, spreadRadius, color)

      let result = BoxShadow::parser()
        .parse_to_end("10px 5px 15px 8px #ff0000")
        .unwrap();

      assert_eq!(result.offset_x.value, 10.0);
      assert_eq!(result.offset_x.unit, "px");
      assert_eq!(result.offset_y.value, 5.0);
      assert_eq!(result.offset_y.unit, "px");
      assert_eq!(result.blur_radius.value, 15.0);
      assert_eq!(result.blur_radius.unit, "px");
      assert_eq!(result.spread_radius.value, 8.0);
      assert_eq!(result.spread_radius.unit, "px");

      if let Color::Hash(hash) = &result.color {
        assert_eq!(hash.value, "ff0000");
      } else {
        stylex_panic!("Expected Hash color");
      }

      assert!(!result.inset);
    }

    #[test]
    fn valid_box_shadow_with_inset_keyword() {
      // Test: Valid: box-shadow with inset keyword

      let result = BoxShadow::parser()
        .parse_to_end("10px 5px #ff0000 inset")
        .unwrap();

      assert_eq!(result.offset_x.value, 10.0);
      assert_eq!(result.offset_x.unit, "px");
      assert_eq!(result.offset_y.value, 5.0);
      assert_eq!(result.offset_y.unit, "px");
      assert_eq!(result.blur_radius.value, 0.0);
      assert_eq!(result.blur_radius.unit, "px");
      assert_eq!(result.spread_radius.value, 0.0);
      assert_eq!(result.spread_radius.unit, "px");

      if let Color::Hash(hash) = &result.color {
        assert_eq!(hash.value, "ff0000");
      } else {
        stylex_panic!("Expected Hash color");
      }

      assert!(result.inset);
    }

    #[test]
    fn valid_box_shadow_with_inset_keyword_and_blur_spread_radius() {
      // Test: Valid: box-shadow with inset keyword and blur/spread radius

      let result = BoxShadow::parser()
        .parse_to_end("10px 5px 15px 8px #ff0000 inset")
        .unwrap();

      assert_eq!(result.offset_x.value, 10.0);
      assert_eq!(result.offset_x.unit, "px");
      assert_eq!(result.offset_y.value, 5.0);
      assert_eq!(result.offset_y.unit, "px");
      assert_eq!(result.blur_radius.value, 15.0);
      assert_eq!(result.blur_radius.unit, "px");
      assert_eq!(result.spread_radius.value, 8.0);
      assert_eq!(result.spread_radius.unit, "px");

      if let Color::Hash(hash) = &result.color {
        assert_eq!(hash.value, "ff0000");
      } else {
        stylex_panic!("Expected Hash color");
      }

      assert!(result.inset);
    }

    #[test]
    fn valid_box_shadow_with_different_length_units() {
      // Test: Valid: box-shadow with different length units

      let result = BoxShadow::parser()
        .parse_to_end("1rem 0.5em 2vw 1vh rgba(0, 0, 0, 0.5)")
        .unwrap();

      assert_eq!(result.offset_x.value, 1.0);
      assert_eq!(result.offset_x.unit, "rem");
      assert_eq!(result.offset_y.value, 0.5);
      assert_eq!(result.offset_y.unit, "em");
      assert_eq!(result.blur_radius.value, 2.0);
      assert_eq!(result.blur_radius.unit, "vw");
      assert_eq!(result.spread_radius.value, 1.0);
      assert_eq!(result.spread_radius.unit, "vh");

      if let Color::Rgba(rgba) = &result.color {
        assert_eq!(rgba.r, 0.0);
        assert_eq!(rgba.g, 0.0);
        assert_eq!(rgba.b, 0.0);
        assert_eq!(rgba.a, 0.5);
      } else {
        stylex_panic!("Expected Rgba color");
      }

      assert!(!result.inset);
    }

    #[test]
    fn valid_box_shadow_with_rgba_color() {
      // Test: Valid: box-shadow with rgba color

      let result = BoxShadow::parser()
        .parse_to_end("10px 5px rgba(255, 0, 0, 0.5)")
        .unwrap();

      assert_eq!(result.offset_x.value, 10.0);
      assert_eq!(result.offset_x.unit, "px");
      assert_eq!(result.offset_y.value, 5.0);
      assert_eq!(result.offset_y.unit, "px");
      assert_eq!(result.blur_radius.value, 0.0);
      assert_eq!(result.blur_radius.unit, "px");
      assert_eq!(result.spread_radius.value, 0.0);
      assert_eq!(result.spread_radius.unit, "px");

      if let Color::Rgba(rgba) = &result.color {
        assert_eq!(rgba.r, 255.0);
        assert_eq!(rgba.g, 0.0);
        assert_eq!(rgba.b, 0.0);
        assert_eq!(rgba.a, 0.5);
      } else {
        stylex_panic!("Expected Rgba color");
      }

      assert!(!result.inset);
    }

    #[test]
    fn valid_box_shadow_with_hsla_color() {
      // Test: Valid: box-shadow with hsla color

      let result = BoxShadow::parser()
        .parse_to_end("10px 5px hsla(0, 100%, 50%, 0.5)")
        .unwrap();

      assert_eq!(result.offset_x.value, 10.0);
      assert_eq!(result.offset_x.unit, "px");
      assert_eq!(result.offset_y.value, 5.0);
      assert_eq!(result.offset_y.unit, "px");
      assert_eq!(result.blur_radius.value, 0.0);
      assert_eq!(result.blur_radius.unit, "px");
      assert_eq!(result.spread_radius.value, 0.0);
      assert_eq!(result.spread_radius.unit, "px");

      if let Color::Hsla(hsla) = &result.color {
        assert_eq!(hsla.h.value, 0.0);
        assert_eq!(hsla.h.unit, "deg");
        assert_eq!(hsla.s.value, 100.0);
        assert_eq!(hsla.l.value, 50.0);
        assert_eq!(hsla.a, 0.5);
      } else {
        stylex_panic!("Expected Hsla color");
      }

      assert!(!result.inset);
    }
  }

  #[cfg(test)]
  mod box_shadow_list {
    use stylex_macros::stylex_panic;

    use super::*;

    #[test]
    fn valid_single_box_shadow() {
      // Test: Valid: single box-shadow

      let result = BoxShadowList::parser()
        .parse_to_end("10px 5px #ff0000")
        .unwrap();

      assert_eq!(result.shadows.len(), 1);

      let shadow = &result.shadows[0];
      assert_eq!(shadow.offset_x.value, 10.0);
      assert_eq!(shadow.offset_x.unit, "px");
      assert_eq!(shadow.offset_y.value, 5.0);
      assert_eq!(shadow.offset_y.unit, "px");
      assert_eq!(shadow.blur_radius.value, 0.0);
      assert_eq!(shadow.blur_radius.unit, "px");
      assert_eq!(shadow.spread_radius.value, 0.0);
      assert_eq!(shadow.spread_radius.unit, "px");

      if let Color::Hash(hash) = &shadow.color {
        assert_eq!(hash.value, "ff0000");
      } else {
        stylex_panic!("Expected Hash color");
      }

      assert!(!shadow.inset);
    }

    #[test]
    fn valid_multiple_box_shadows() {
      // Test: Valid: multiple box-shadows

      let result = BoxShadowList::parser()
        .parse_to_end(
          "10px 5px #ff0000, 5px 5px 10px #00ff00, 0 0 15px 5px rgba(0, 0, 255, 0.5) inset",
        )
        .unwrap();

      assert_eq!(result.shadows.len(), 3);

      // First shadow
      let shadow1 = &result.shadows[0];
      assert_eq!(shadow1.offset_x.value, 10.0);
      assert_eq!(shadow1.offset_x.unit, "px");
      assert_eq!(shadow1.offset_y.value, 5.0);
      assert_eq!(shadow1.offset_y.unit, "px");
      assert_eq!(shadow1.blur_radius.value, 0.0);
      assert_eq!(shadow1.spread_radius.value, 0.0);
      if let Color::Hash(hash) = &shadow1.color {
        assert_eq!(hash.value, "ff0000");
      }
      assert!(!shadow1.inset);

      // Second shadow
      let shadow2 = &result.shadows[1];
      assert_eq!(shadow2.offset_x.value, 5.0);
      assert_eq!(shadow2.offset_y.value, 5.0);
      assert_eq!(shadow2.blur_radius.value, 10.0);
      assert_eq!(shadow2.spread_radius.value, 0.0);
      if let Color::Hash(hash) = &shadow2.color {
        assert_eq!(hash.value, "00ff00");
      }
      assert!(!shadow2.inset);

      // Third shadow (inset)
      let shadow3 = &result.shadows[2];
      assert_eq!(shadow3.offset_x.value, 0.0);
      assert_eq!(shadow3.offset_y.value, 0.0);
      assert_eq!(shadow3.blur_radius.value, 15.0);
      assert_eq!(shadow3.spread_radius.value, 5.0);
      if let Color::Rgba(rgba) = &shadow3.color {
        assert_eq!(rgba.r, 0.0);
        assert_eq!(rgba.g, 0.0);
        assert_eq!(rgba.b, 255.0);
        assert_eq!(rgba.a, 0.5);
      }
      assert!(shadow3.inset);
    }

    #[test]
    fn valid_box_shadows_with_whitespace_around_commas() {
      // Test: Valid: box-shadows with whitespace around commas

      let result = BoxShadowList::parser()
        .parse_to_end("10px 5px #ff0000 , 5px 5px 10px #00ff00")
        .unwrap();

      assert_eq!(result.shadows.len(), 2);

      // First shadow
      let shadow1 = &result.shadows[0];
      assert_eq!(shadow1.offset_x.value, 10.0);
      assert_eq!(shadow1.offset_y.value, 5.0);
      if let Color::Hash(hash) = &shadow1.color {
        assert_eq!(hash.value, "ff0000");
      }

      // Second shadow
      let shadow2 = &result.shadows[1];
      assert_eq!(shadow2.offset_x.value, 5.0);
      assert_eq!(shadow2.offset_y.value, 5.0);
      assert_eq!(shadow2.blur_radius.value, 10.0);
      if let Color::Hash(hash) = &shadow2.color {
        assert_eq!(hash.value, "00ff00");
      }
    }
  }

  #[cfg(test)]
  mod invalid_cases {
    use super::*;

    #[test]
    fn invalid_missing_required_parameters() {
      // Test: Invalid: missing required parameters

      let result = BoxShadow::parser().parse_to_end("10px");
      assert!(result.is_err(), "Should fail for missing parameters");

      let result = BoxShadow::parser().parse_to_end("#ff0000");
      assert!(result.is_err(), "Should fail for missing offset values");
    }

    #[test]
    fn invalid_incorrect_order_of_parameters() {
      // Test: Invalid: incorrect order of parameters

      let result = BoxShadow::parser().parse_to_end("#ff0000 10px 5px");
      assert!(result.is_err(), "Should fail for color before offsets");

      // So we test a different invalid case instead
      let result = BoxShadow::parser().parse_to_end("10px inset 5px #ff0000");
      assert!(result.is_err(), "Should fail for inset in wrong position");
    }

    #[test]
    fn invalid_malformed_values() {
      // Test: Invalid: malformed values

      let result = BoxShadow::parser().parse_to_end("10px 5px #ff00");
      assert!(result.is_err(), "Should fail for invalid hex color");

      let result = BoxShadow::parser().parse_to_end("10 5px #ff0000");
      assert!(result.is_err(), "Should fail for missing unit");
    }
  }
}

#[cfg(test)]
mod test_css_property_box_shadow_list {
  use super::*;

  #[test]
  fn box_shadow_list_single() {
    let shadow_list = BoxShadowList::parser().parse_to_end("5px 5px red").unwrap();

    assert_eq!(shadow_list.shadows.len(), 1);

    let shadow = &shadow_list.shadows[0];
    assert_eq!(shadow.offset_x.value, 5.0);
    assert_eq!(shadow.offset_y.value, 5.0);

    assert_eq!(shadow_list.to_string(), "5px 5px red");
  }

  #[test]
  fn box_shadow_list_multiple() {
    let shadow_list = BoxShadowList::parser()
      .parse_to_end("5px 5px red, inset 2px 2px blue")
      .unwrap();

    assert_eq!(shadow_list.shadows.len(), 2);

    // First shadow
    let first = &shadow_list.shadows[0];
    assert!(!first.inset);
    assert_eq!(first.offset_x.value, 5.0);
    assert_eq!(first.offset_y.value, 5.0);

    // Second shadow
    let second = &shadow_list.shadows[1];
    assert!(second.inset);
    assert_eq!(second.offset_x.value, 2.0);
    assert_eq!(second.offset_y.value, 2.0);

    assert_eq!(shadow_list.to_string(), "5px 5px red, inset 2px 2px blue");
  }

  #[test]
  fn box_shadow_list_complex() {
    let input = "0 1px 3px rgba(0,0,0,0.12), 0 1px 2px rgba(0,0,0,0.24), inset 0 0 0 1px #e0e0e0";
    let shadow_list = BoxShadowList::parser().parse_to_end(input).unwrap();

    assert_eq!(shadow_list.shadows.len(), 3);

    // Verify each shadow parsed correctly
    assert!(!shadow_list.shadows[0].inset);
    assert!(!shadow_list.shadows[1].inset);
    assert!(shadow_list.shadows[2].inset);

    // Verify round-trip parsing
    let reparsed = BoxShadowList::parser()
      .parse_to_end(&shadow_list.to_string())
      .unwrap();
    assert_eq!(shadow_list.shadows.len(), reparsed.shadows.len());
  }

  #[test]

  fn box_shadow_none() {
    let shadow_list = BoxShadowList::parser().parse_to_end("none").unwrap();
    assert!(shadow_list.shadows.is_empty());
    assert_eq!(shadow_list.to_string(), "none");
  }

  #[test]
  fn box_shadow_list_whitespace_handling() {
    let whitespace_cases = vec![
      "5px 5px red,inset 2px 2px blue",          // No space after comma
      "5px 5px red , inset 2px 2px blue",        // Space before comma
      "5px  5px  red  ,  inset  2px  2px  blue", // Extra spaces
    ];

    for input in whitespace_cases {
      let shadow_list = BoxShadowList::parser().parse_to_end(input).unwrap();
      assert_eq!(shadow_list.shadows.len(), 2, "Should parse: {}", input);
    }
  }

  #[test]
  fn box_shadow_list_edge_cases() {
    // Single shadow that looks like it could be multiple
    let single_complex = BoxShadowList::parser()
      .parse_to_end("inset 0 0 0 1px rgba(0,0,0,0.1)")
      .unwrap();
    assert_eq!(single_complex.shadows.len(), 1);
    assert!(single_complex.shadows[0].inset);

    // Multiple shadows with different complexities
    let mixed = BoxShadowList::parser()
      .parse_to_end("2px 2px red, 0 0 5px 2px blue, inset 1px 1px black")
      .unwrap();
    assert_eq!(mixed.shadows.len(), 3);
  }

  #[test]
  fn box_shadow_none_comprehensive() {
    // Test "none" keyword parsing and formatting
    let shadow_list = BoxShadowList::parser().parse_to_end("none").unwrap();
    assert!(shadow_list.shadows.is_empty());
    assert_eq!(shadow_list.to_string(), "none");

    // Test that empty shadow list displays as "none"
    let empty_list = BoxShadowList::new(vec![]);
    assert_eq!(empty_list.to_string(), "none");

    // Test round-trip parsing
    let reparsed = BoxShadowList::parser()
      .parse_to_end(&shadow_list.to_string())
      .unwrap();
    assert!(reparsed.shadows.is_empty());
    assert_eq!(reparsed.to_string(), "none");
  }

  #[test]
  fn box_shadow_none_edge_cases() {
    // Test "none" with surrounding whitespace
    let whitespace_cases = vec![
      " none",  // Leading whitespace
      "none ",  // Trailing whitespace
      " none ", // Both leading and trailing
    ];

    for input in whitespace_cases {
      let result = BoxShadowList::parser().parse_to_end(input);
      // These should fail because we don't handle whitespace around "none" currently
      // This is consistent with how CSS parsers typically work
      assert!(result.is_err(), "Should fail for: '{}'", input);
    }

    // Test invalid cases that might be mistaken for "none"
    let invalid_none_cases = vec![
      "None",              // Wrong case
      "NONE",              // Wrong case
      "none,",             // Trailing comma
      "none, 5px 5px red", // "none" shouldn't be mixed with other shadows
      "5px 5px red, none", // "none" shouldn't be mixed with other shadows
    ];

    for invalid_input in invalid_none_cases {
      let result = BoxShadowList::parser().parse_to_end(invalid_input);
      assert!(result.is_err(), "Should fail for: '{}'", invalid_input);
    }
  }

  #[test]
  fn invalid_box_shadow_list() {
    let invalid_cases = vec![
      "",                     // Empty string
      "5px",                  // Incomplete
      "5px 5px red,",         // Trailing comma
      "5px 5px red, invalid", // Invalid second shadow
      "invalid, 5px 5px red", // Invalid first shadow
    ];

    for invalid_input in invalid_cases {
      let result = BoxShadowList::parser().parse_to_end(invalid_input);
      assert!(result.is_err(), "Should fail for: {}", invalid_input);
    }
  }
}
