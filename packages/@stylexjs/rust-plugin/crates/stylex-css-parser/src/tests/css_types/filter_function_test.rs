/*!
Filter function tests.

Note: This test suite provides comprehensive filter function parsing coverage.
*/

use crate::css_types::filter_function::FilterFunction;

#[cfg(test)]
mod filter_function_parse {
  use super::*;

  #[test]
  fn parses_blur_filter() {
    let result = FilterFunction::parser().parse_to_end("blur(5px)").unwrap();
    assert!(result.to_string().contains("blur"));
  }

  #[test]
  fn parses_brightness_filter() {
    let result = FilterFunction::parser()
      .parse_to_end("brightness(150%)")
      .unwrap();
    assert!(result.to_string().contains("brightness"));
  }

  #[test]
  fn parses_contrast_filter() {
    let result = FilterFunction::parser()
      .parse_to_end("contrast(200%)")
      .unwrap();
    assert!(result.to_string().contains("contrast"));
  }

  #[test]
  fn parses_grayscale_filter() {
    let result = FilterFunction::parser()
      .parse_to_end("grayscale(50%)")
      .unwrap();
    assert!(result.to_string().contains("grayscale"));
  }

  #[test]
  fn parses_hue_rotate_filter() {
    let result = FilterFunction::parser()
      .parse_to_end("hue-rotate(90deg)")
      .unwrap();
    assert!(result.to_string().contains("hue-rotate"));
  }

  #[test]
  fn parses_invert_filter() {
    let result = FilterFunction::parser()
      .parse_to_end("invert(100%)")
      .unwrap();
    assert!(result.to_string().contains("invert"));
  }

  #[test]
  fn parses_opacity_filter() {
    let result = FilterFunction::parser()
      .parse_to_end("opacity(50%)")
      .unwrap();
    assert!(result.to_string().contains("opacity"));
  }

  #[test]
  fn parses_saturate_filter() {
    let result = FilterFunction::parser()
      .parse_to_end("saturate(200%)")
      .unwrap();
    assert!(result.to_string().contains("saturate"));
  }

  #[test]
  fn parses_sepia_filter() {
    let result = FilterFunction::parser()
      .parse_to_end("sepia(100%)")
      .unwrap();
    assert!(result.to_string().contains("sepia"));
  }

  #[test]
  fn rejects_invalid_filter_functions() {
    assert!(
      FilterFunction::parser()
        .parse_to_end("invalid-filter(50%)")
        .is_err()
    );
    assert!(FilterFunction::parser().parse_to_end("blur()").is_err());
    assert!(
      FilterFunction::parser()
        .parse_to_end("brightness(abc)")
        .is_err()
    );
    assert!(
      FilterFunction::parser()
        .parse_to_end("brightness(-50%)")
        .is_err()
    );
    assert!(FilterFunction::parser().parse_to_end("").is_err());
  }

  #[test]
  fn blur_to_string() {
    assert_eq!(
      FilterFunction::parser()
        .parse_to_end("blur(5px)")
        .unwrap()
        .to_string(),
      "blur(5px)"
    );
  }

  #[test]
  fn brightness_number_to_string() {
    assert_eq!(
      FilterFunction::parser()
        .parse_to_end("brightness(1.5)")
        .unwrap()
        .to_string(),
      "brightness(1.5)"
    );
  }

  #[test]
  fn brightness_percentage_to_string() {
    // 150% is parsed as 150/100 = 1.5
    assert_eq!(
      FilterFunction::parser()
        .parse_to_end("brightness(150%)")
        .unwrap()
        .to_string(),
      "brightness(1.5)"
    );
  }

  #[test]
  fn contrast_to_string() {
    // 200% is parsed as 200/100 = 2
    assert_eq!(
      FilterFunction::parser()
        .parse_to_end("contrast(200%)")
        .unwrap()
        .to_string(),
      "contrast(2)"
    );
  }

  #[test]
  fn saturate_to_string() {
    // 200% is parsed as 200/100 = 2
    assert_eq!(
      FilterFunction::parser()
        .parse_to_end("saturate(200%)")
        .unwrap()
        .to_string(),
      "saturate(2)"
    );
  }

  #[test]
  fn hue_rotate_to_string() {
    assert_eq!(
      FilterFunction::parser()
        .parse_to_end("hue-rotate(90deg)")
        .unwrap()
        .to_string(),
      "hue-rotate(90deg)"
    );
  }

  #[test]
  fn grayscale_percentage_to_string() {
    // 100% is parsed as 100/100 = 1
    assert_eq!(
      FilterFunction::parser()
        .parse_to_end("grayscale(100%)")
        .unwrap()
        .to_string(),
      "grayscale(1)"
    );
  }

  #[test]
  fn grayscale_number_to_string() {
    assert_eq!(
      FilterFunction::parser()
        .parse_to_end("grayscale(0.5)")
        .unwrap()
        .to_string(),
      "grayscale(0.5)"
    );
  }

  #[test]
  fn invert_to_string() {
    // 100% is parsed as 100/100 = 1
    assert_eq!(
      FilterFunction::parser()
        .parse_to_end("invert(100%)")
        .unwrap()
        .to_string(),
      "invert(1)"
    );
  }

  #[test]
  fn opacity_percentage_to_string() {
    // 50% is parsed as 50/100 = 0.5
    assert_eq!(
      FilterFunction::parser()
        .parse_to_end("opacity(50%)")
        .unwrap()
        .to_string(),
      "opacity(0.5)"
    );
  }

  #[test]
  fn opacity_number_to_string() {
    assert_eq!(
      FilterFunction::parser()
        .parse_to_end("opacity(0.5)")
        .unwrap()
        .to_string(),
      "opacity(0.5)"
    );
  }

  #[test]
  fn sepia_to_string() {
    // 100% is parsed as 100/100 = 1
    assert_eq!(
      FilterFunction::parser()
        .parse_to_end("sepia(100%)")
        .unwrap()
        .to_string(),
      "sepia(1)"
    );
  }
}
