// Tests extracted for css_types/angle_percentage.rs behaviors and edge cases.
// Source: crates/stylex-css-parser/src/css_types/angle_percentage.rs

use super::*;

#[test]
fn test_angle_percentage_creation() {
  let angle = AnglePercentage::Angle(Angle::new(45.0, "deg".to_string()));
  let percentage = AnglePercentage::Percentage(Percentage::new(50.0));

  assert!(angle.is_angle());
  assert!(!angle.is_percentage());

  assert!(!percentage.is_angle());
  assert!(percentage.is_percentage());
}

#[test]
fn test_angle_percentage_display() {
  let angle = AnglePercentage::Angle(Angle::new(90.0, "deg".to_string()));
  assert_eq!(angle.to_string(), "90deg");

  let percentage = AnglePercentage::Percentage(Percentage::new(25.0));
  assert_eq!(percentage.to_string(), "25%");

  let rad_angle = AnglePercentage::Angle(Angle::new(std::f64::consts::PI, "rad".to_string()));
  assert_eq!(
    rad_angle.to_string(),
    format!("{}rad", std::f64::consts::PI)
  );

  let zero_angle = AnglePercentage::Angle(Angle::new(0.0, "deg".to_string()));
  assert_eq!(zero_angle.to_string(), "0deg");
}

#[test]
fn test_angle_percentage_accessors() {
  let angle_val = Angle::new(180.0, "deg".to_string());
  let percentage_val = Percentage::new(75.0);

  let angle = AnglePercentage::Angle(angle_val.clone());
  let percentage = AnglePercentage::Percentage(percentage_val.clone());

  // Test as_angle
  assert_eq!(angle.as_angle(), Some(&angle_val));
  assert_eq!(percentage.as_angle(), None);

  // Test as_percentage
  assert_eq!(angle.as_percentage(), None);
  assert_eq!(percentage.as_percentage(), Some(&percentage_val));
}

#[test]
fn test_angle_percentage_parser_creation() {
  // Test both creation methods
  let _parser1 = AnglePercentage::parser();
  let _parser2 = angle_percentage_parser();
}

#[test]
fn test_angle_percentage_equality() {
  let angle1 = AnglePercentage::Angle(Angle::new(45.0, "deg".to_string()));
  let angle2 = AnglePercentage::Angle(Angle::new(45.0, "deg".to_string()));
  let angle3 = AnglePercentage::Angle(Angle::new(90.0, "deg".to_string()));

  let percentage1 = AnglePercentage::Percentage(Percentage::new(50.0));
  let percentage2 = AnglePercentage::Percentage(Percentage::new(50.0));

  assert_eq!(angle1, angle2);
  assert_ne!(angle1, angle3);
  assert_ne!(angle1, percentage1);
  assert_eq!(percentage1, percentage2);
}

#[test]
fn test_angle_percentage_different_units() {
  // Test different angle units
  let deg_angle = AnglePercentage::Angle(Angle::new(90.0, "deg".to_string()));
  let rad_angle = AnglePercentage::Angle(Angle::new(1.57, "rad".to_string()));
  let grad_angle = AnglePercentage::Angle(Angle::new(100.0, "grad".to_string()));
  let turn_angle = AnglePercentage::Angle(Angle::new(0.25, "turn".to_string()));

  assert_eq!(deg_angle.to_string(), "90deg");
  assert_eq!(rad_angle.to_string(), "1.57rad");
  assert_eq!(grad_angle.to_string(), "100grad");
  assert_eq!(turn_angle.to_string(), "0.25turn");

  // All should be different values
  assert_ne!(deg_angle, rad_angle);
  assert_ne!(deg_angle, grad_angle);
  assert_ne!(deg_angle, turn_angle);
}

#[test]
fn test_angle_percentage_edge_cases() {
  // Zero angle
  let zero_angle = AnglePercentage::Angle(Angle::new(0.0, "deg".to_string()));
  assert!(zero_angle.is_angle());
  assert_eq!(zero_angle.to_string(), "0deg");

  // Zero percentage
  let zero_percentage = AnglePercentage::Percentage(Percentage::new(0.0));
  assert!(zero_percentage.is_percentage());
  assert_eq!(zero_percentage.to_string(), "0%");

  // 100% percentage
  let full_percentage = AnglePercentage::Percentage(Percentage::new(100.0));
  assert_eq!(full_percentage.to_string(), "100%");

  // Full turn
  let full_turn = AnglePercentage::Angle(Angle::new(1.0, "turn".to_string()));
  assert_eq!(full_turn.to_string(), "1turn");
}

#[test]
fn test_angle_percentage_common_use_cases() {
  // Common use cases in CSS gradients, transforms, etc.

  // Gradient angle
  let gradient_angle = AnglePercentage::Angle(Angle::new(45.0, "deg".to_string()));
  assert!(gradient_angle.is_angle());

  // Conic gradient percentage
  let conic_percentage = AnglePercentage::Percentage(Percentage::new(25.0));
  assert!(conic_percentage.is_percentage());

  // Transform rotate angle
  let rotate_angle = AnglePercentage::Angle(Angle::new(0.5, "turn".to_string()));
  assert!(rotate_angle.is_angle());

  // Skew angle
  let skew_angle = AnglePercentage::Angle(Angle::new(30.0, "deg".to_string()));
  assert!(skew_angle.is_angle());
}
