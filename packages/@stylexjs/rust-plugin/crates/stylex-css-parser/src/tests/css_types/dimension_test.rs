/*!
CSS Dimension Tests

Test CSS Type: <dimension>
Tests parsing of dimensional values (length, time, frequency, resolution).
*/

#[cfg(test)]
mod test_css_type_dimension {
  use stylex_macros::stylex_panic;

  use crate::css_types::{
    dimension::{Dimension, dimension},
    frequency::Frequency,
    length::Length,
    resolution::Resolution,
    time::Time,
  };

  #[test]
  fn parses_length_dimensions() {
    // Test various length units
    let result = dimension().parse_to_end("16px").unwrap();
    if let Dimension::Length(length) = result {
      assert_eq!(length.value, 16.0);
      assert_eq!(length.unit, "px");
    } else {
      stylex_panic!("Expected Length dimension");
    }

    let result = dimension().parse_to_end("2.5em").unwrap();
    if let Dimension::Length(length) = result {
      assert_eq!(length.value, 2.5);
      assert_eq!(length.unit, "em");
    } else {
      stylex_panic!("Expected Length dimension");
    }

    let result = dimension().parse_to_end("100vh").unwrap();
    if let Dimension::Length(length) = result {
      assert_eq!(length.value, 100.0);
      assert_eq!(length.unit, "vh");
    } else {
      stylex_panic!("Expected Length dimension");
    }
  }

  #[test]
  fn parses_time_dimensions() {
    // Test time units
    let result = dimension().parse_to_end("1.5s").unwrap();
    if let Dimension::Time(time) = result {
      assert_eq!(time.value, 1.5);
      assert_eq!(time.unit, "s");
    } else {
      stylex_panic!("Expected Time dimension");
    }

    let result = dimension().parse_to_end("300ms").unwrap();
    if let Dimension::Time(time) = result {
      assert_eq!(time.value, 300.0);
      assert_eq!(time.unit, "ms");
    } else {
      stylex_panic!("Expected Time dimension");
    }
  }

  #[test]
  fn parses_frequency_dimensions() {
    // Test frequency units
    let result = dimension().parse_to_end("440Hz").unwrap();
    if let Dimension::Frequency(freq) = result {
      assert_eq!(freq.value, 440.0);
      assert_eq!(freq.unit, "Hz");
    } else {
      stylex_panic!("Expected Frequency dimension");
    }

    let result = dimension().parse_to_end("20KHz").unwrap();
    if let Dimension::Frequency(freq) = result {
      assert_eq!(freq.value, 20.0);
      assert_eq!(freq.unit, "KHz");
    } else {
      stylex_panic!("Expected Frequency dimension");
    }
  }

  #[test]
  fn parses_resolution_dimensions() {
    // Test resolution units
    let result = dimension().parse_to_end("96dpi").unwrap();
    if let Dimension::Resolution(res) = result {
      assert_eq!(res.value, 96.0);
      assert_eq!(res.unit, "dpi");
    } else {
      stylex_panic!("Expected Resolution dimension");
    }

    let result = dimension().parse_to_end("2dppx").unwrap();
    if let Dimension::Resolution(res) = result {
      assert_eq!(res.value, 2.0);
      assert_eq!(res.unit, "dppx");
    } else {
      stylex_panic!("Expected Resolution dimension");
    }

    let result = dimension().parse_to_end("38dpcm").unwrap();
    if let Dimension::Resolution(res) = result {
      assert_eq!(res.value, 38.0);
      assert_eq!(res.unit, "dpcm");
    } else {
      stylex_panic!("Expected Resolution dimension");
    }
  }

  #[test]
  fn rejects_invalid_dimensions() {
    // Invalid units should fail to parse
    assert!(dimension().parse_to_end("10invalid").is_err());
    assert!(dimension().parse_to_end("5deg").is_err()); // Angle units are not dimensions
    assert!(dimension().parse_to_end("50%").is_err()); // Percentages are not dimensions
    assert!(dimension().parse_to_end("red").is_err()); // Colors are not dimensions
  }

  #[test]
  fn dimension_display_formatting() {
    // Test that dimensions display correctly
    let length = Dimension::Length(Length::new(16.0, "px".to_string()));
    assert_eq!(length.to_string(), "16px");

    let time = Dimension::Time(Time::new(1.5, "s".to_string()));
    assert_eq!(time.to_string(), "1.5s");

    let freq = Dimension::Frequency(Frequency::new(440.0, "Hz".to_string()));
    assert_eq!(freq.to_string(), "0.44KHz");

    let res = Dimension::Resolution(Resolution::new(96.0, "dpi".to_string()));
    assert_eq!(res.to_string(), "96dpi");
  }
}
