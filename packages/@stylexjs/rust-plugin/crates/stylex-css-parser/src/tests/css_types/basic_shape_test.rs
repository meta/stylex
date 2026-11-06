/*!
Basic shape CSS type tests.

Test CSS Type: <basic-shape>
Tests parsing of inset(), circle(), ellipse(), polygon(), and path() functions.
*/

use crate::css_types::{
  basic_shape::{BasicShape, CircleRadius},
  length::Length,
  length_percentage::LengthPercentage,
};

#[cfg(test)]
mod test_css_type_basic_shape {
  use super::*;

  mod inset {}

  mod circle {
    use stylex_macros::stylex_panic;

    use super::*;

    #[test]
    fn should_parse_valid_circles() {
      let result = BasicShape::parse().parse_to_end("circle(10px)").unwrap();
      if let BasicShape::Circle { radius, position } = result {
        assert_eq!(
          radius,
          CircleRadius::Length(LengthPercentage::Length(Length::new(
            10.0,
            "px".to_string()
          )))
        );
        assert_eq!(position, None);
      } else {
        stylex_panic!("Expected Circle variant");
      }

      let result = BasicShape::parse()
        .parse_to_end("circle(closest-side)")
        .unwrap();
      if let BasicShape::Circle { radius, position } = result {
        assert_eq!(radius, CircleRadius::ClosestSide);
        assert_eq!(position, None);
      } else {
        stylex_panic!("Expected Circle variant");
      }

      let result = BasicShape::parse()
        .parse_to_end("circle(farthest-side)")
        .unwrap();
      if let BasicShape::Circle { radius, position } = result {
        assert_eq!(radius, CircleRadius::FarthestSide);
        assert_eq!(position, None);
      } else {
        stylex_panic!("Expected Circle variant");
      }
    }

    #[test]
    fn should_not_parse_invalid_circles() {
      assert!(BasicShape::parse().parse_to_end("circle(invalid)").is_err());
      assert!(
        BasicShape::parse()
          .parse_to_end("circle(10px, invalid)")
          .is_err()
      );
    }
  }

  mod ellipse {}

  mod polygon {
    use super::*;

    #[test]
    fn should_not_parse_invalid_polygons() {
      assert!(
        BasicShape::parse()
          .parse_to_end("polygon(invalid)")
          .is_err()
      );
      assert!(
        BasicShape::parse()
          .parse_to_end("polygon(0% 0%, invalid)")
          .is_err()
      );
    }
  }

  mod path {
    use super::*;

    #[test]
    fn should_parse_path_with_fill_rule() {
      let result = BasicShape::parse().parse_to_end("path(evenodd, \"M0,0 L100,100\")");
      if let Ok(BasicShape::Path { fill_rule, path }) = result {
        assert_eq!(path, "M0,0 L100,100");
        assert_eq!(fill_rule, Some("evenodd".to_string()));
      }
    }
  }

  #[test]
  fn test_inset_string_representation() {
    let inset = BasicShape::Inset {
      top: LengthPercentage::Length(Length::new(10.0, "px".to_string())),
      right: LengthPercentage::Length(Length::new(10.0, "px".to_string())),
      bottom: LengthPercentage::Length(Length::new(10.0, "px".to_string())),
      left: LengthPercentage::Length(Length::new(10.0, "px".to_string())),
      round: None,
    };
    assert_eq!(inset.to_string(), "inset(10px)");
  }

  #[test]
  fn test_keyword_radius_string_representation() {
    let circle = BasicShape::Circle {
      radius: CircleRadius::ClosestSide,
      position: None,
    };
    assert_eq!(circle.to_string(), "circle(closest-side)");

    let ellipse = BasicShape::Ellipse {
      radius_x: CircleRadius::FarthestSide,
      radius_y: CircleRadius::ClosestSide,
      position: None,
    };
    assert_eq!(ellipse.to_string(), "ellipse(farthest-side closest-side)");
  }

  mod parse_to_string {
    use super::*;

    #[test]
    fn circle_with_position() {
      assert_eq!(
        BasicShape::parse()
          .parse_to_end("circle(50px at center center)")
          .unwrap()
          .to_string(),
        "circle(50px at center center)"
      );
    }

    #[test]
    fn ellipse_with_position() {
      assert_eq!(
        BasicShape::parse()
          .parse_to_end("ellipse(50px 30px at 10% 20%)")
          .unwrap()
          .to_string(),
        "ellipse(50px 30px at 10% 20%)"
      );
    }

    #[test]
    fn polygon_with_fill_rule() {
      let result = BasicShape::parse()
        .parse_to_end("polygon(evenodd, 0% 0%, 100% 0%, 50% 100%)")
        .unwrap();
      assert!(result.to_string().contains("polygon"));
      if let BasicShape::Polygon { fill_rule, points } = result {
        assert_eq!(points.len(), 3);
        assert_eq!(fill_rule, Some("evenodd".to_string()));
      }
    }

    #[test]
    fn inset_four_values() {
      assert_eq!(
        BasicShape::parse()
          .parse_to_end("inset(10px 20px 30px 40px)")
          .unwrap()
          .to_string(),
        "inset(10px 20px 30px 40px)"
      );
    }

    #[test]
    fn path_to_string() {
      let result = BasicShape::parse()
        .parse_to_end("path(\"M 0 0 L 100 100\")")
        .unwrap();
      assert!(result.to_string().contains("path"));
      if let BasicShape::Path { path, .. } = result {
        assert_eq!(path, "M 0 0 L 100 100");
      }
    }
  }
}
