/*!
CSS Alpha Value Tests

Test CSS Type: <alpha-value>
Tests number fractions, percentages, and rejection of invalid values.
*/

#[cfg(test)]
mod test_css_type_alpha_value {
  use crate::css_types::alpha_value::AlphaValue;

  mod number_fractions {
    use super::*;

    #[test]
    fn _05() {
      let result = AlphaValue::parser().parse_to_end("0.5").unwrap();
      assert_eq!(result.value, 0.5);
    }

    #[test]
    fn _5() {
      let result = AlphaValue::parser().parse_to_end(".5").unwrap();
      assert_eq!(result.value, 0.5);
    }

    #[test]
    fn _025() {
      let result = AlphaValue::parser().parse_to_end("0.25").unwrap();
      assert_eq!(result.value, 0.25);
    }

    #[test]
    fn _25() {
      let result = AlphaValue::parser().parse_to_end(".25").unwrap();
      assert_eq!(result.value, 0.25);
    }

    #[test]
    fn _075() {
      let result = AlphaValue::parser().parse_to_end("0.75").unwrap();
      assert_eq!(result.value, 0.75);
    }

    #[test]
    fn _75() {
      let result = AlphaValue::parser().parse_to_end(".75").unwrap();
      assert_eq!(result.value, 0.75);
    }

    #[test]
    fn _1() {
      let result = AlphaValue::parser().parse_to_end("1").unwrap();
      assert_eq!(result.value, 1.0);
    }

    #[test]
    fn parses_decimal_alpha_values() {
      let result = AlphaValue::parser().parse_to_end("0").unwrap();
      assert_eq!(result.value, 0.0);

      let result = AlphaValue::parser().parse_to_end("0.25").unwrap();
      assert_eq!(result.value, 0.25);

      let result = AlphaValue::parser().parse_to_end("0.5").unwrap();
      assert_eq!(result.value, 0.5);

      let result = AlphaValue::parser().parse_to_end("1").unwrap();
      assert_eq!(result.value, 1.0);
    }
  }

  mod percentages {
    use super::*;

    #[test]
    fn _50_percent() {
      let result = AlphaValue::parser().parse_to_end("50%").unwrap();
      assert_eq!(result.value, 0.5);
    }

    #[test]
    fn _25_percent() {
      let result = AlphaValue::parser().parse_to_end("25%").unwrap();
      assert_eq!(result.value, 0.25);
    }

    #[test]
    fn _75_percent() {
      let result = AlphaValue::parser().parse_to_end("75%").unwrap();
      assert_eq!(result.value, 0.75);
    }

    #[test]
    fn _75_5_percent() {
      let result = AlphaValue::parser().parse_to_end("75.5%").unwrap();
      assert_eq!(result.value, 0.755);
    }

    #[test]
    fn _0_25_percent() {
      let result = AlphaValue::parser().parse_to_end("0.25%").unwrap();
      assert_eq!(result.value, 0.0025);
    }

    #[test]
    fn _25_percent_dot() {
      let result = AlphaValue::parser().parse_to_end(".25%").unwrap();
      assert_eq!(result.value, 0.0025);
    }

    #[test]
    fn parses_percentage_alpha_values() {
      let result = AlphaValue::parser().parse_to_end("0%").unwrap();
      assert_eq!(result.value, 0.0);

      let result = AlphaValue::parser().parse_to_end("25%").unwrap();
      assert_eq!(result.value, 0.25);

      let result = AlphaValue::parser().parse_to_end("50%").unwrap();
      assert_eq!(result.value, 0.5);

      let result = AlphaValue::parser().parse_to_end("100%").unwrap();
      assert_eq!(result.value, 1.0);
    }
  }

  mod rejects {
    use super::*;

    #[test]
    fn rejects_invalid_alpha_values() {
      assert!(AlphaValue::parser().parse_to_end("invalid").is_err());
      assert!(AlphaValue::parser().parse_to_end("red").is_err());
      assert!(AlphaValue::parser().parse_to_end("initial").is_err());
    }
  }
}
