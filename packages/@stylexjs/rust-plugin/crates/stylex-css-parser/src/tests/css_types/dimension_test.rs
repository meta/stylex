/*!
CSS Dimension Tests

Test CSS Type: <dimension>
Tests parsing of dimensional values (length, time, frequency, resolution).
*/

#[cfg(test)]
mod test_css_type_dimension {

  use crate::css_types::{
    dimension::{Dimension, dimension},
    frequency::Frequency,
    length::Length,
    resolution::Resolution,
    time::Time,
  };

  #[test]
  fn rejects_invalid_dimensions() {
    assert!(dimension().parse_to_end("10invalid").is_err());
    assert!(dimension().parse_to_end("5deg").is_err()); // Angle units are not dimensions
    assert!(dimension().parse_to_end("50%").is_err()); // Percentages are not dimensions
    assert!(dimension().parse_to_end("red").is_err()); // Colors are not dimensions
  }

  #[test]
  fn dimension_display_formatting() {
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
