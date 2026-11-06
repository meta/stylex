use super::*;

#[test]
fn test_angle_percentage_accessors() {
  let angle_val = Angle::new(180.0, "deg".to_string());
  let percentage_val = Percentage::new(75.0);

  let angle = AnglePercentage::Angle(angle_val.clone());
  let percentage = AnglePercentage::Percentage(percentage_val.clone());

  assert_eq!(angle.as_angle(), Some(&angle_val));
  assert_eq!(percentage.as_angle(), None);

  assert_eq!(angle.as_percentage(), None);
  assert_eq!(percentage.as_percentage(), Some(&percentage_val));
}

#[test]
fn test_angle_percentage_parser_creation() {
  let _parser1 = AnglePercentage::parser();
  let _parser2 = angle_percentage_parser();
}

#[test]
fn test_angle_percentage_edge_cases() {
  let zero_angle = AnglePercentage::Angle(Angle::new(0.0, "deg".to_string()));
  assert!(zero_angle.is_angle());
  assert_eq!(zero_angle.to_string(), "0deg");

  let zero_percentage = AnglePercentage::Percentage(Percentage::new(0.0));
  assert!(zero_percentage.is_percentage());
  assert_eq!(zero_percentage.to_string(), "0%");

  let full_percentage = AnglePercentage::Percentage(Percentage::new(100.0));
  assert_eq!(full_percentage.to_string(), "100%");

  let full_turn = AnglePercentage::Angle(Angle::new(1.0, "turn".to_string()));
  assert_eq!(full_turn.to_string(), "1turn");
}
