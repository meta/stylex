/*!
CSS Angle Tests

Test CSS Type: <angle>
Tests parsing of degree, radian, gradian, and turn units.
*/

#[cfg(test)]
mod test_css_type_angle {
  use crate::css_types::angle::Angle;

  #[test]
  fn parses_css_angle_types_strings_correctly() {
    let result = Angle::parser().parse_to_end("0deg").unwrap();
    assert_eq!(result.value, 0.0);
    assert_eq!(result.unit, "deg");

    let result = Angle::parser().parse_to_end("45deg").unwrap();
    assert_eq!(result.value, 45.0);
    assert_eq!(result.unit, "deg");

    let result = Angle::parser().parse_to_end("90deg").unwrap();
    assert_eq!(result.value, 90.0);
    assert_eq!(result.unit, "deg");

    let result = Angle::parser().parse_to_end("180deg").unwrap();
    assert_eq!(result.value, 180.0);
    assert_eq!(result.unit, "deg");

    let result = Angle::parser().parse_to_end("270deg").unwrap();
    assert_eq!(result.value, 270.0);
    assert_eq!(result.unit, "deg");

    let result = Angle::parser().parse_to_end("-90deg").unwrap();
    assert_eq!(result.value, -90.0);
    assert_eq!(result.unit, "deg");

    let result = Angle::parser().parse_to_end("0.5turn").unwrap();
    assert_eq!(result.value, 0.5);
    assert_eq!(result.unit, "turn");

    let result = Angle::parser().parse_to_end("2rad").unwrap();
    assert_eq!(result.value, 2.0);
    assert_eq!(result.unit, "rad");

    let result = Angle::parser().parse_to_end("100grad").unwrap();
    assert_eq!(result.value, 100.0);
    assert_eq!(result.unit, "grad");

    let result = Angle::parser().parse_to_end("1.5deg").unwrap();
    assert_eq!(result.value, 1.5);
    assert_eq!(result.unit, "deg");
  }

  mod rejects {
    use super::*;

    #[test]
    fn rejects_invalid_angle_values() {
      assert!(Angle::parser().parse_to_end("invalid").is_err());
      assert!(Angle::parser().parse_to_end("red").is_err());
      assert!(Angle::parser().parse_to_end("initial").is_err());
    }
  }
}
