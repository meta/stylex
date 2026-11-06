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
