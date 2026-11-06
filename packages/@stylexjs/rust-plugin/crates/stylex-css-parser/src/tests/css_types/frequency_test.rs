/*!
CSS Frequency Tests

Test CSS frequency types with Hz and KHz units.
*/

#[cfg(test)]
mod frequency_parse {
  use crate::css_types::frequency::Frequency;

  #[test]
  fn parses_valid_css_frequency_types_strings_correctly() {
    let result = Frequency::parser().parse_to_end("1Hz").unwrap();
    assert_eq!(result.value, 1.0);
    assert_eq!(result.unit, "Hz");

    let result = Frequency::parser().parse_to_end("1000KHz").unwrap();
    assert_eq!(result.value, 1000.0);
    assert_eq!(result.unit, "KHz");

    let result = Frequency::parser().parse_to_end("0Hz").unwrap();
    assert_eq!(result.value, 0.0);
    assert_eq!(result.unit, "Hz");

    let result = Frequency::parser().parse_to_end("0KHz").unwrap();
    assert_eq!(result.value, 0.0);
    assert_eq!(result.unit, "KHz");

    let result = Frequency::parser().parse_to_end("1.5Hz").unwrap();
    assert_eq!(result.value, 1.5);
    assert_eq!(result.unit, "Hz");

    let result = Frequency::parser().parse_to_end("1.5KHz").unwrap();
    assert_eq!(result.value, 1.5);
    assert_eq!(result.unit, "KHz");
  }

  #[test]
  fn fails_to_parse_invalid_css_frequency_types_strings() {
    assert!(Frequency::parser().parse_to_end("1 Hz").is_err());
    assert!(Frequency::parser().parse_to_end("1KHz ").is_err());
    assert!(Frequency::parser().parse_to_end("1").is_err());
    assert!(Frequency::parser().parse_to_end("Hz").is_err());
    assert!(Frequency::parser().parse_to_end("KHz").is_err());
  }
}
