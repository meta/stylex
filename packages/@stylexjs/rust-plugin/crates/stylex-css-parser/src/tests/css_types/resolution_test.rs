/*!
CSS Resolution Tests

Test CSS resolution types with dpi, dpcm, and dppx units.
*/

#[cfg(test)]
mod test_css_type_resolution {
  use crate::css_types::resolution::Resolution;

  #[test]
  fn parses_dpi_values() {
    let result = Resolution::parser().parse_to_end("300dpi").unwrap();
    assert_eq!(result.value, 300.0);
    assert_eq!(result.unit, "dpi");
  }

  #[test]
  fn parses_dpcm_values() {
    let result = Resolution::parser().parse_to_end("118.11dpcm").unwrap();
    assert_eq!(result.value, 118.11);
    assert_eq!(result.unit, "dpcm");
  }

  #[test]
  fn parses_dppx_values() {
    let result = Resolution::parser().parse_to_end("96dppx").unwrap();
    assert_eq!(result.value, 96.0);
    assert_eq!(result.unit, "dppx");
  }

  #[test]
  fn rejects_invalid_resolution_values() {
    assert!(Resolution::parser().parse_to_end("invalid").is_err());
    assert!(Resolution::parser().parse_to_end("10abc").is_err());
    assert!(Resolution::parser().parse_to_end("10").is_err());
  }
}
