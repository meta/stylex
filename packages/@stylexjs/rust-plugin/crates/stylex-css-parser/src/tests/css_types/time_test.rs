/*!
CSS Time Tests

Test CSS time types with s and ms units.
*/

#[cfg(test)]
mod time_parse {
  use crate::css_types::time::Time;

  #[test]
  fn parses_valid_css_time_types_strings_correctly() {
    let result = Time::parser().parse_to_end("1s").unwrap();
    assert_eq!(result.value, 1.0);
    assert_eq!(result.unit, "s");

    let result = Time::parser().parse_to_end("1000ms").unwrap();
    assert_eq!(result.value, 1000.0);
    assert_eq!(result.unit, "ms");

    let result = Time::parser().parse_to_end("0s").unwrap();
    assert_eq!(result.value, 0.0);
    assert_eq!(result.unit, "s");

    let result = Time::parser().parse_to_end("0ms").unwrap();
    assert_eq!(result.value, 0.0);
    assert_eq!(result.unit, "ms");

    let result = Time::parser().parse_to_end("1.5s").unwrap();
    assert_eq!(result.value, 1.5);
    assert_eq!(result.unit, "s");

    let result = Time::parser().parse_to_end("1.5ms").unwrap();
    assert_eq!(result.value, 1.5);
    assert_eq!(result.unit, "ms");
  }

  #[test]
  fn fails_to_parse_invalid_css_time_types_strings() {
    assert!(Time::parser().parse_to_end("1 s").is_err());
    assert!(Time::parser().parse_to_end("1ms ").is_err());
    assert!(Time::parser().parse_to_end("1").is_err());
    assert!(Time::parser().parse_to_end("s").is_err());
    assert!(Time::parser().parse_to_end("ms").is_err());
  }
}
