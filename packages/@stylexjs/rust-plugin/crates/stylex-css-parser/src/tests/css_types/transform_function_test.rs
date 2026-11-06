/*!
Transform function tests.
*/

use crate::css_types::transform_function::TransformFunction;

#[cfg(test)]
mod test_css_type_transform_function {
  use super::*;

  #[cfg(test)]
  mod test_matrix_function {
    use super::*;

    #[test]
    fn valid_uses() {
      assert_eq!(
        TransformFunction::parse()
          .parse_to_end("matrix(1, 0, 0, 1, 0, 0)")
          .unwrap()
          .to_string(),
        "matrix(1, 0, 0, 1, 0, 0)"
      );
      assert_eq!(
        TransformFunction::parse()
          .parse_to_end("matrix(1.2,0.2,  -1, 0.9, 0, 20 )")
          .unwrap()
          .to_string(),
        "matrix(1.2, 0.2, -1, 0.9, 0, 20)"
      );
      assert_eq!(
        TransformFunction::parse()
          .parse_to_end("matrix(\n.4,0,0.5,1.200,60,10   )")
          .unwrap()
          .to_string(),
        "matrix(0.4, 0, 0.5, 1.2, 60, 10)"
      );
      assert_eq!(
        TransformFunction::parse()
          .parse_to_end("matrix(0.1, 1, -0.3, 1, 0, 0)")
          .unwrap()
          .to_string(),
        "matrix(0.1, 1, -0.3, 1, 0, 0)"
      );
    }

    #[test]
    fn invalid_uses() {
      // Not enough values
      assert!(
        TransformFunction::parse()
          .parse_to_end("matrix(1, 0, 0, 1, 0)")
          .is_err()
      );
      // Too many values
      assert!(
        TransformFunction::parse()
          .parse_to_end("matrix(1, 0, 0, 1, 0, 0, 0)")
          .is_err()
      );
      // Non-numeric values
      assert!(
        TransformFunction::parse()
          .parse_to_end("matrix(1, 0, 0, 1, 0, foo)")
          .is_err()
      );
      // wrong type of values
      assert!(
        TransformFunction::parse()
          .parse_to_end("matrix(1px, 0, 0, 1, 0, 0)")
          .is_err()
      );
      // wrong separator
      assert!(
        TransformFunction::parse()
          .parse_to_end("matrix(1 0 0 1 0 0)")
          .is_err()
      );
    }
  }

  #[cfg(test)]
  mod test_matrix3d_function {
    use super::*;

    #[test]
    fn valid_uses() {
      let result = TransformFunction::parse()
        .parse_to_end("matrix3d(1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1)")
        .unwrap();
      assert!(result.to_string().contains("matrix3d"));
    }

    #[test]
    fn invalid_uses() {
      assert!(
        TransformFunction::parse()
          .parse_to_end("matrix3d()")
          .is_err()
      );
    }
  }

  #[cfg(test)]
  mod test_perspective_function {
    use super::*;

    #[test]
    fn valid_uses() {
      assert_eq!(
        TransformFunction::parse()
          .parse_to_end("perspective(0)")
          .unwrap()
          .to_string(),
        "perspective(0)"
      );
      assert_eq!(
        TransformFunction::parse()
          .parse_to_end("perspective(100px)")
          .unwrap()
          .to_string(),
        "perspective(100px)"
      );
      assert_eq!(
        TransformFunction::parse()
          .parse_to_end("perspective(1.5em)")
          .unwrap()
          .to_string(),
        "perspective(1.5em)"
      );
    }

    #[test]
    fn invalid_uses() {
      // Non-numeric values
      assert!(
        TransformFunction::parse()
          .parse_to_end("perspective(foo)")
          .is_err()
      );
      // wrong type of values
      assert!(
        TransformFunction::parse()
          .parse_to_end("perspective(1)")
          .is_err()
      );
      assert!(
        TransformFunction::parse()
          .parse_to_end("perspective(1%)")
          .is_err()
      );
      assert!(
        TransformFunction::parse()
          .parse_to_end("perspective(1deg)")
          .is_err()
      );
    }
  }

  #[cfg(test)]
  mod test_rotate_function {
    use super::*;

    #[test]
    fn valid_uses() {
      assert_eq!(
        TransformFunction::parse()
          .parse_to_end("rotate(0)")
          .unwrap()
          .to_string(),
        "rotate(0deg)"
      );
      assert_eq!(
        TransformFunction::parse()
          .parse_to_end("rotate(45deg)")
          .unwrap()
          .to_string(),
        "rotate(45deg)"
      );
      assert_eq!(
        TransformFunction::parse()
          .parse_to_end("rotate(90deg)")
          .unwrap()
          .to_string(),
        "rotate(90deg)"
      );
      assert_eq!(
        TransformFunction::parse()
          .parse_to_end("rotate(180deg)")
          .unwrap()
          .to_string(),
        "rotate(180deg)"
      );
      assert_eq!(
        TransformFunction::parse()
          .parse_to_end("rotate(270deg)")
          .unwrap()
          .to_string(),
        "rotate(270deg)"
      );
      assert_eq!(
        TransformFunction::parse()
          .parse_to_end("rotate(-90deg)")
          .unwrap()
          .to_string(),
        "rotate(-90deg)"
      );
      assert_eq!(
        TransformFunction::parse()
          .parse_to_end("rotate(0.5turn)")
          .unwrap()
          .to_string(),
        "rotate(0.5turn)"
      );
      assert_eq!(
        TransformFunction::parse()
          .parse_to_end("rotate(2rad)")
          .unwrap()
          .to_string(),
        "rotate(2rad)"
      );
      assert_eq!(
        TransformFunction::parse()
          .parse_to_end("rotate(100grad)")
          .unwrap()
          .to_string(),
        "rotate(100grad)"
      );
      assert_eq!(
        TransformFunction::parse()
          .parse_to_end("rotate(1.5deg)")
          .unwrap()
          .to_string(),
        "rotate(1.5deg)"
      );
      assert_eq!(
        TransformFunction::parse()
          .parse_to_end("rotate(360deg)")
          .unwrap()
          .to_string(),
        "rotate(360deg)"
      );
      assert_eq!(
        TransformFunction::parse()
          .parse_to_end("rotate(1turn)")
          .unwrap()
          .to_string(),
        "rotate(1turn)"
      );
      assert_eq!(
        TransformFunction::parse()
          .parse_to_end("rotate(-1turn)")
          .unwrap()
          .to_string(),
        "rotate(-1turn)"
      );
    }

    #[test]
    fn invalid_uses() {
      // Non-numeric values
      assert!(
        TransformFunction::parse()
          .parse_to_end("rotate(foo)")
          .is_err()
      );
      // wrong type of values
      assert!(
        TransformFunction::parse()
          .parse_to_end("rotate(1)")
          .is_err()
      );
      assert!(
        TransformFunction::parse()
          .parse_to_end("rotate(1%)")
          .is_err()
      );
      assert!(
        TransformFunction::parse()
          .parse_to_end("rotate(1px)")
          .is_err()
      );
    }
  }

  #[cfg(test)]
  mod test_rotate3d_function {
    use super::*;

    #[test]
    fn valid_uses() {
      let result = TransformFunction::parse()
        .parse_to_end("rotate3d(1, 0, 0, 90deg)")
        .unwrap();
      assert_eq!(result.to_string(), "rotateX(90deg)");
    }

    #[test]
    fn invalid_uses() {
      assert!(
        TransformFunction::parse()
          .parse_to_end("rotate3d()")
          .is_err()
      );
    }
  }

  #[cfg(test)]
  mod test_rotate_axis_function {
    use super::*;

    #[test]
    fn valid_uses() {
      let result = TransformFunction::parse()
        .parse_to_end("rotateX(90deg)")
        .unwrap();
      assert!(result.to_string().contains("rotateX"));

      let result = TransformFunction::parse()
        .parse_to_end("rotateY(-45deg)")
        .unwrap();
      assert!(result.to_string().contains("rotateY"));

      let result = TransformFunction::parse()
        .parse_to_end("rotateZ(180deg)")
        .unwrap();
      assert!(result.to_string().contains("rotateZ"));
    }

    #[test]
    fn invalid_uses() {
      assert!(
        TransformFunction::parse()
          .parse_to_end("rotateX()")
          .is_err()
      );
      assert!(
        TransformFunction::parse()
          .parse_to_end("rotateY(90)")
          .is_err()
      );
      assert!(
        TransformFunction::parse()
          .parse_to_end("rotateZ(90px)")
          .is_err()
      );
    }
  }

  #[cfg(test)]
  mod test_scale_function {
    use super::*;

    #[test]
    fn valid_uses() {
      let result = TransformFunction::parse().parse_to_end("scale(2)").unwrap();
      assert!(result.to_string().contains("scale"));

      let result = TransformFunction::parse()
        .parse_to_end("scale(1.5, 0.8)")
        .unwrap();
      assert!(result.to_string().contains("scale"));

      let result = TransformFunction::parse()
        .parse_to_end("scale(0.5)")
        .unwrap();
      assert!(result.to_string().contains("scale"));
    }

    #[test]
    fn invalid_uses() {
      assert!(TransformFunction::parse().parse_to_end("scale()").is_err());
      assert!(
        TransformFunction::parse()
          .parse_to_end("scale(1, 2, 3)")
          .is_err()
      );
    }
  }

  #[cfg(test)]
  mod test_translate_function {
    use super::*;

    #[test]
    fn valid_uses() {
      let result = TransformFunction::parse()
        .parse_to_end("translate(50px)")
        .unwrap();
      assert!(result.to_string().contains("translate"));

      let result = TransformFunction::parse()
        .parse_to_end("translate(10px, 20px)")
        .unwrap();
      assert!(result.to_string().contains("translate"));

      let result = TransformFunction::parse()
        .parse_to_end("translate(50%, 25%)")
        .unwrap();
      assert!(result.to_string().contains("translate"));
    }

    #[test]
    fn invalid_uses() {
      assert!(
        TransformFunction::parse()
          .parse_to_end("translate()")
          .is_err()
      );
      assert!(
        TransformFunction::parse()
          .parse_to_end("translate(10)")
          .is_err()
      );
    }
  }

  #[cfg(test)]
  mod test_matrix3d_parse_to_string {
    use super::*;

    #[test]
    fn identity_matrix3d() {
      assert_eq!(
        TransformFunction::parse()
          .parse_to_end("matrix3d(1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1)")
          .unwrap()
          .to_string(),
        "matrix3d(1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1)"
      );
    }
  }

  #[cfg(test)]
  mod test_translate3d_parse_to_string {
    use super::*;

    #[test]
    fn translate3d_with_px_values() {
      assert_eq!(
        TransformFunction::parse()
          .parse_to_end("translate3d(10px, 20px, 30px)")
          .unwrap()
          .to_string(),
        "translate3d(10px, 20px, 30px)"
      );
    }
  }

  #[cfg(test)]
  mod test_rotate3d_parse_to_string {
    use super::*;

    #[test]
    fn rotate3d_x_axis() {
      // rotate3d(1, 0, 0, ...) is optimized to rotateX(...)
      assert_eq!(
        TransformFunction::parse()
          .parse_to_end("rotate3d(1, 0, 0, 45deg)")
          .unwrap()
          .to_string(),
        "rotateX(45deg)"
      );
    }
  }

  #[cfg(test)]
  mod test_scale3d_parse_to_string {
    use super::*;

    #[test]
    fn scale3d_values() {
      assert_eq!(
        TransformFunction::parse()
          .parse_to_end("scale3d(1, 2, 3)")
          .unwrap()
          .to_string(),
        "scale3d(1, 2, 3)"
      );
    }
  }

  #[cfg(test)]
  mod test_perspective_parse_to_string {
    use super::*;

    #[test]
    fn perspective_100px() {
      assert_eq!(
        TransformFunction::parse()
          .parse_to_end("perspective(100px)")
          .unwrap()
          .to_string(),
        "perspective(100px)"
      );
    }
  }

  #[cfg(test)]
  mod test_skew_parse_to_string {
    use super::*;

    #[test]
    fn skew_single_angle() {
      assert_eq!(
        TransformFunction::parse()
          .parse_to_end("skew(30deg)")
          .unwrap()
          .to_string(),
        "skew(30deg)"
      );
    }

    #[test]
    fn skew_two_angles() {
      assert_eq!(
        TransformFunction::parse()
          .parse_to_end("skew(30deg, 20deg)")
          .unwrap()
          .to_string(),
        "skew(30deg, 20deg)"
      );
    }
  }

  #[cfg(test)]
  mod test_skew_axis_parse_to_string {
    use super::*;

    #[test]
    fn skew_x() {
      assert_eq!(
        TransformFunction::parse()
          .parse_to_end("skewX(30deg)")
          .unwrap()
          .to_string(),
        "skewX(30deg)"
      );
    }

    #[test]
    fn skew_y() {
      assert_eq!(
        TransformFunction::parse()
          .parse_to_end("skewY(20deg)")
          .unwrap()
          .to_string(),
        "skewY(20deg)"
      );
    }
  }
}
