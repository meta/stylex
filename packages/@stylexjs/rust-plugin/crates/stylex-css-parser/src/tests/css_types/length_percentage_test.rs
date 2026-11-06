/*!
CSS Length Percentage Tests

Test CSS length-percentage type that accepts both lengths and percentages.
*/

#[cfg(test)]
mod test_css_type_length_percentage {
  use stylex_macros::stylex_panic;

  use crate::css_types::length_percentage::{LengthPercentage, length_percentage_parser};

  #[test]
  fn parses_length_values() {
    let result = length_percentage_parser().parse_to_end("10px").unwrap();
    match result {
      LengthPercentage::Length(length) => {
        assert_eq!(length.value, 10.0);
        assert_eq!(length.unit, "px");
      },
      _ => stylex_panic!("Expected length"),
    }

    let result = length_percentage_parser().parse_to_end("5em").unwrap();
    match result {
      LengthPercentage::Length(length) => {
        assert_eq!(length.value, 5.0);
        assert_eq!(length.unit, "em");
      },
      _ => stylex_panic!("Expected length"),
    }

    let result = length_percentage_parser().parse_to_end("2rem").unwrap();
    match result {
      LengthPercentage::Length(length) => {
        assert_eq!(length.value, 2.0);
        assert_eq!(length.unit, "rem");
      },
      _ => stylex_panic!("Expected length"),
    }

    let result = length_percentage_parser().parse_to_end("1in").unwrap();
    match result {
      LengthPercentage::Length(length) => {
        assert_eq!(length.value, 1.0);
        assert_eq!(length.unit, "in");
      },
      _ => stylex_panic!("Expected length"),
    }
  }

  #[test]
  fn parses_percentage_values() {
    let result = length_percentage_parser().parse_to_end("50%").unwrap();
    match result {
      LengthPercentage::Percentage(percentage) => {
        assert_eq!(percentage.value, 50.0);
      },
      _ => stylex_panic!("Expected percentage"),
    }

    let result = length_percentage_parser().parse_to_end("100%").unwrap();
    match result {
      LengthPercentage::Percentage(percentage) => {
        assert_eq!(percentage.value, 100.0);
      },
      _ => stylex_panic!("Expected percentage"),
    }

    let result = length_percentage_parser().parse_to_end("0%").unwrap();
    match result {
      LengthPercentage::Percentage(percentage) => {
        assert_eq!(percentage.value, 0.0);
      },
      _ => stylex_panic!("Expected percentage"),
    }

    let result = length_percentage_parser().parse_to_end("25%").unwrap();
    match result {
      LengthPercentage::Percentage(percentage) => {
        assert_eq!(percentage.value, 25.0);
      },
      _ => stylex_panic!("Expected percentage"),
    }
  }

  #[test]
  fn parses_calc_expressions() {
    let result = length_percentage_parser()
      .parse_to_end("calc(10px + 5%)")
      .unwrap();
    match result {
      LengthPercentage::Calc(calc) => {
        assert_eq!(calc.to_string(), "calc(10px + 5%)");
      },
      _ => stylex_panic!("Expected calc expression"),
    }

    let result = length_percentage_parser()
      .parse_to_end("calc(100% - 20px)")
      .unwrap();
    match result {
      LengthPercentage::Calc(calc) => {
        assert_eq!(calc.to_string(), "calc(100% - 20px)");
      },
      _ => stylex_panic!("Expected calc expression"),
    }

    let result = length_percentage_parser()
      .parse_to_end("calc(50px * 2)")
      .unwrap();
    match result {
      LengthPercentage::Calc(calc) => {
        assert!(!calc.to_string().is_empty());
        assert!(calc.to_string().starts_with("calc("));
        assert!(calc.to_string().ends_with(")"));
        assert_eq!(calc.to_string(), "calc(50px * 2)");
      },
      _ => stylex_panic!("Expected calc expression"),
    }
  }

  #[test]
  fn rejects_invalid_length_percentage_values() {
    assert!(length_percentage_parser().parse_to_end("abc").is_err());
    assert!(length_percentage_parser().parse_to_end("50").is_err());
    assert!(length_percentage_parser().parse_to_end("10abc").is_err());
  }
}
