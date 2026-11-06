// Tests extracted for css_types/basic_shape.rs behaviors and edge cases.
// Source: crates/stylex-css-parser/src/css_types/basic_shape.rs

use super::*;

#[test]
fn test_circle_radius() {
  let result = BasicShape::parse().parse_to_end("circle(closest-side)");
  assert!(result.is_ok());
  assert_eq!(
    result.unwrap(),
    BasicShape::Circle {
      radius: CircleRadius::ClosestSide,
      position: None,
    }
  );
}

#[test]
fn test_basic_shape_display() {
  let circle = BasicShape::Circle {
    radius: CircleRadius::ClosestSide,
    position: None,
  };
  assert_eq!(circle.to_string(), "circle(closest-side)");
}

#[test]
fn test_inset_display_optimization() {
  let inset = BasicShape::Inset {
    top: LengthPercentage::Length(crate::css_types::Length::new(10.0, "px".to_string())),
    right: LengthPercentage::Length(crate::css_types::Length::new(10.0, "px".to_string())),
    bottom: LengthPercentage::Length(crate::css_types::Length::new(10.0, "px".to_string())),
    left: LengthPercentage::Length(crate::css_types::Length::new(10.0, "px".to_string())),
    round: None,
  };
  assert_eq!(inset.to_string(), "inset(10px)");
}

#[test]
fn test_polygon_display() {
  let polygon = BasicShape::Polygon {
    fill_rule: Some("nonzero".to_string()),
    points: vec![(
      LengthPercentage::Length(crate::css_types::Length::new(0.0, "px".to_string())),
      LengthPercentage::Length(crate::css_types::Length::new(0.0, "px".to_string())),
    )],
  };
  assert!(polygon.to_string().contains("polygon(nonzero"));
}

#[test]
fn test_path_display() {
  let path = BasicShape::Path {
    fill_rule: Some("evenodd".to_string()),
    path: "M 0 0 L 100 100".to_string(),
  };
  assert_eq!(path.to_string(), "path(evenodd, \"M 0 0 L 100 100\")");
}

#[test]
fn test_ellipse_two_radii() {
  let ellipse = BasicShape::Ellipse {
    radius_x: CircleRadius::Length(LengthPercentage::Length(crate::css_types::Length::new(
      50.0,
      "px".to_string(),
    ))),
    radius_y: CircleRadius::Length(LengthPercentage::Length(crate::css_types::Length::new(
      25.0,
      "px".to_string(),
    ))),
    position: None,
  };
  assert_eq!(ellipse.to_string(), "ellipse(50px 25px)");
}
