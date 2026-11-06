// Tests extracted for properties/transform.rs behaviors and edge cases.
// Source: crates/stylex-css-parser/src/properties/transform.rs

use super::*;
use crate::css_types::{Angle, Length, LengthPercentage, Percentage, transform_function::*};

#[test]
fn test_transform_function_creation() {
  let translate = TransformFunction::TranslateAxis(TranslateAxis::new(
    LengthPercentage::Length(Length::new(10.0, "px".to_string())),
    Axis::X,
  ));

  // Test that it can be created without issues
  assert!(matches!(translate, TransformFunction::TranslateAxis(_)));
}

#[test]
fn test_transform_function_display() {
  let translate_x = TransformFunction::TranslateAxis(TranslateAxis::new(
    LengthPercentage::Length(Length::new(10.0, "px".to_string())),
    Axis::X,
  ));
  assert_eq!(translate_x.to_string(), "translateX(10px)");

  let translate = TransformFunction::Translate(Translate::new(
    LengthPercentage::Length(Length::new(10.0, "px".to_string())),
    Some(LengthPercentage::Length(Length::new(
      20.0,
      "px".to_string(),
    ))),
  ));
  assert_eq!(translate.to_string(), "translate(10px, 20px)");

  let rotate = TransformFunction::Rotate(Rotate::new(Angle::new(45.0, "deg".to_string())));
  assert_eq!(rotate.to_string(), "rotate(45deg)");
}

#[test]
fn test_transform_creation() {
  let func1 = TransformFunction::TranslateAxis(TranslateAxis::new(
    LengthPercentage::Length(Length::new(10.0, "px".to_string())),
    Axis::X,
  ));
  let func2 = TransformFunction::Rotate(Rotate::new(Angle::new(45.0, "deg".to_string())));

  let transform = Transform::new(vec![func1, func2]);
  assert_eq!(transform.value.len(), 2);
}

#[test]
fn test_transform_display() {
  let func1 = TransformFunction::TranslateAxis(TranslateAxis::new(
    LengthPercentage::Length(Length::new(10.0, "px".to_string())),
    Axis::X,
  ));
  let func2 = TransformFunction::Rotate(Rotate::new(Angle::new(45.0, "deg".to_string())));

  let transform = Transform::new(vec![func1, func2]);
  assert_eq!(transform.to_string(), "translateX(10px) rotate(45deg)");
}

#[test]
fn test_transform_empty() {
  let transform = Transform::new(vec![]);
  assert_eq!(transform.to_string(), "");
}

#[test]
fn test_transform_equality() {
  let func = TransformFunction::ScaleAxis(ScaleAxis::new(1.5, Axis::X));

  let transform1 = Transform::new(vec![func.clone()]);
  let transform2 = Transform::new(vec![func]);

  assert_eq!(transform1, transform2);
}

#[test]
fn test_transform_parser_creation() {
  // Test that the parser can be created without panicking
  let _parser = Transform::parser();
}

#[test]
fn test_transform_single_function() {
  let transform = Transform::new(vec![
    TransformFunction::Translate(Translate::new(
      LengthPercentage::Percentage(Percentage::new(50.0)),
      Some(LengthPercentage::Percentage(Percentage::new(-50.0))),
    )),
    TransformFunction::Rotate(Rotate::new(Angle::new(45.0, "deg".to_string()))),
    TransformFunction::Scale(Scale::new(1.2, Some(1.2))),
  ]);

  assert_eq!(transform.value.len(), 3);
  assert!(transform.to_string().contains("translate(50%, -50%)"));
  assert!(transform.to_string().contains("rotate(45deg)"));
  assert!(transform.to_string().contains("scale(1.2, 1.2)"));
}

#[test]
fn test_transform_complex_combination() {
  let func1 = TransformFunction::Translate(Translate::new(
    LengthPercentage::Length(Length::new(10.0, "px".to_string())),
    Some(LengthPercentage::Length(Length::new(
      20.0,
      "px".to_string(),
    ))),
  ));
  let func2 = TransformFunction::Rotate(Rotate::new(Angle::new(45.0, "deg".to_string())));
  let func3 = TransformFunction::Scale(Scale::new(1.2, Some(1.2)));

  let transform = Transform::new(vec![func1, func2, func3]);
  assert_eq!(transform.value.len(), 3);
  assert_eq!(
    transform.to_string(),
    "translate(10px, 20px) rotate(45deg) scale(1.2, 1.2)"
  );
}

#[test]
fn test_transform_common_functions() {
  // Test various common transform functions
  let translate = TransformFunction::Translate(Translate::new(
    LengthPercentage::Length(Length::new(10.0, "px".to_string())),
    Some(LengthPercentage::Length(Length::new(
      20.0,
      "px".to_string(),
    ))),
  ));

  let rotate = TransformFunction::Rotate(Rotate::new(Angle::new(45.0, "deg".to_string())));

  let scale = TransformFunction::Scale(Scale::new(1.5, Some(1.5)));

  let skew = TransformFunction::Skew(Skew::new(
    Angle::new(10.0, "deg".to_string()),
    Some(Angle::new(20.0, "deg".to_string())),
  ));

  let functions = vec![translate, rotate, scale, skew];
  let transform = Transform::new(functions);

  assert_eq!(transform.value.len(), 4);
  assert!(transform.to_string().contains("translate"));
  assert!(transform.to_string().contains("rotate"));
  assert!(transform.to_string().contains("scale"));
  assert!(transform.to_string().contains("skew"));
}
