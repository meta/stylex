/*!
CSS Length Tests

Test CSS Type: <length>
Tests parsing of all CSS length units (font, viewport, container, absolute).
*/

#[cfg(test)]
mod test_css_type_length {
  use crate::css_types::length::Length;

  #[test]
  fn parses_font_based_units() {
    let result = Length::parser().parse_to_end("10ch").unwrap();
    assert_eq!(result.value, 10.0);
    assert_eq!(result.unit, "ch");

    let result = Length::parser().parse_to_end("2em").unwrap();
    assert_eq!(result.value, 2.0);
    assert_eq!(result.unit, "em");

    let result = Length::parser().parse_to_end("1.5ex").unwrap();
    assert_eq!(result.value, 1.5);
    assert_eq!(result.unit, "ex");

    let result = Length::parser().parse_to_end("3ic").unwrap();
    assert_eq!(result.value, 3.0);
    assert_eq!(result.unit, "ic");

    let result = Length::parser().parse_to_end("1.2lh").unwrap();
    assert_eq!(result.value, 1.2);
    assert_eq!(result.unit, "lh");

    let result = Length::parser().parse_to_end("2rem").unwrap();
    assert_eq!(result.value, 2.0);
    assert_eq!(result.unit, "rem");

    let result = Length::parser().parse_to_end("1rlh").unwrap();
    assert_eq!(result.value, 1.0);
    assert_eq!(result.unit, "rlh");
  }

  #[test]
  fn parses_viewport_based_units() {
    let result = Length::parser().parse_to_end("50vh").unwrap();
    assert_eq!(result.value, 50.0);
    assert_eq!(result.unit, "vh");

    let result = Length::parser().parse_to_end("100vw").unwrap();
    assert_eq!(result.value, 100.0);
    assert_eq!(result.unit, "vw");

    let result = Length::parser().parse_to_end("80svh").unwrap();
    assert_eq!(result.value, 80.0);
    assert_eq!(result.unit, "svh");

    let result = Length::parser().parse_to_end("90lvw").unwrap();
    assert_eq!(result.value, 90.0);
    assert_eq!(result.unit, "lvw");

    let result = Length::parser().parse_to_end("70dvh").unwrap();
    assert_eq!(result.value, 70.0);
    assert_eq!(result.unit, "dvh");

    let result = Length::parser().parse_to_end("60vmin").unwrap();
    assert_eq!(result.value, 60.0);
    assert_eq!(result.unit, "vmin");

    let result = Length::parser().parse_to_end("85vmax").unwrap();
    assert_eq!(result.value, 85.0);
    assert_eq!(result.unit, "vmax");
  }

  #[test]
  fn parses_container_based_units() {
    let result = Length::parser().parse_to_end("30cqw").unwrap();
    assert_eq!(result.value, 30.0);
    assert_eq!(result.unit, "cqw");

    let result = Length::parser().parse_to_end("40cqi").unwrap();
    assert_eq!(result.value, 40.0);
    assert_eq!(result.unit, "cqi");

    let result = Length::parser().parse_to_end("50cqh").unwrap();
    assert_eq!(result.value, 50.0);
    assert_eq!(result.unit, "cqh");

    let result = Length::parser().parse_to_end("60cqb").unwrap();
    assert_eq!(result.value, 60.0);
    assert_eq!(result.unit, "cqb");

    let result = Length::parser().parse_to_end("45cqmin").unwrap();
    assert_eq!(result.value, 45.0);
    assert_eq!(result.unit, "cqmin");

    let result = Length::parser().parse_to_end("75cqmax").unwrap();
    assert_eq!(result.value, 75.0);
    assert_eq!(result.unit, "cqmax");
  }

  #[test]
  fn parses_absolute_units() {
    let result = Length::parser().parse_to_end("16px").unwrap();
    assert_eq!(result.value, 16.0);
    assert_eq!(result.unit, "px");

    let result = Length::parser().parse_to_end("2cm").unwrap();
    assert_eq!(result.value, 2.0);
    assert_eq!(result.unit, "cm");

    let result = Length::parser().parse_to_end("10mm").unwrap();
    assert_eq!(result.value, 10.0);
    assert_eq!(result.unit, "mm");

    let result = Length::parser().parse_to_end("1in").unwrap();
    assert_eq!(result.value, 1.0);
    assert_eq!(result.unit, "in");

    let result = Length::parser().parse_to_end("12pt").unwrap();
    assert_eq!(result.value, 12.0);
    assert_eq!(result.unit, "pt");
  }

  #[test]
  fn rejects_invalid_units() {
    assert!(Length::parser().parse_to_end("10abc").is_err());
    assert!(Length::parser().parse_to_end("20pc").is_err()); // 'pc' is not supported
    assert!(Length::parser().parse_to_end("30").is_err());
    assert!(Length::parser().parse_to_end("xyz").is_err());
  }
}
