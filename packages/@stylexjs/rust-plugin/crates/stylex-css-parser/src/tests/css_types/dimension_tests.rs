use super::*;

#[test]
fn test_dimension_from_value_and_unit() {
  let length_dim = Dimension::from_value_and_unit(16.0, "px".to_string());
  assert!(matches!(length_dim, Some(Dimension::Length(_))));

  let time_dim = Dimension::from_value_and_unit(1.5, "s".to_string());
  assert!(matches!(time_dim, Some(Dimension::Time(_))));

  let freq_dim = Dimension::from_value_and_unit(440.0, "Hz".to_string());
  assert!(matches!(freq_dim, Some(Dimension::Frequency(_))));

  let res_dim = Dimension::from_value_and_unit(96.0, "dpi".to_string());
  assert!(matches!(res_dim, Some(Dimension::Resolution(_))));

  let invalid_dim = Dimension::from_value_and_unit(10.0, "invalid".to_string());
  assert!(invalid_dim.is_none());
}

#[test]
fn test_dimension_display() {
  let length = Dimension::Length(Length::new(16.0, "px".to_string()));
  assert_eq!(length.to_string(), "16px");

  let time = Dimension::Time(Time::new(1.5, "s".to_string()));
  assert_eq!(time.to_string(), "1.5s");

  let freq = Dimension::Frequency(Frequency::new(440.0, "Hz".to_string()));
  assert_eq!(freq.to_string(), "0.44KHz");

  let res = Dimension::Resolution(Resolution::new(96.0, "dpi".to_string()));
  assert_eq!(res.to_string(), "96dpi");
}

#[test]
fn test_is_valid_dimension_unit() {
  assert!(Dimension::is_valid_dimension_unit("px"));
  assert!(Dimension::is_valid_dimension_unit("em"));
  assert!(Dimension::is_valid_dimension_unit("vh"));

  assert!(Dimension::is_valid_dimension_unit("s"));
  assert!(Dimension::is_valid_dimension_unit("ms"));

  assert!(Dimension::is_valid_dimension_unit("Hz"));
  assert!(Dimension::is_valid_dimension_unit("KHz"));

  assert!(Dimension::is_valid_dimension_unit("dpi"));
  assert!(Dimension::is_valid_dimension_unit("dpcm"));
  assert!(Dimension::is_valid_dimension_unit("dppx"));

  assert!(!Dimension::is_valid_dimension_unit("invalid"));
  assert!(!Dimension::is_valid_dimension_unit("deg")); // Angle units belong to `Angle`.
}

#[test]
fn test_dimension_parser_creation() {
  let _parser = Dimension::parse();
}

#[test]
fn test_dimension_equality() {
  let dim1 = Dimension::Length(Length::new(16.0, "px".to_string()));
  let dim2 = Dimension::Length(Length::new(16.0, "px".to_string()));
  let dim3 = Dimension::Length(Length::new(20.0, "px".to_string()));

  assert_eq!(dim1, dim2);
  assert_ne!(dim1, dim3);
}
