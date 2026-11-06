/*!
Transform property tests.
*/

use crate::{css_types::transform_function::*, properties::transform::Transform};

#[cfg(test)]
mod test_css_property_transform {
  use super::*;

  #[cfg(test)]
  mod single_functions {
    use stylex_macros::stylex_panic;

    use super::*;

    #[test]
    fn matrix() {
      // Test: Transform.parse.parse('matrix(1, 0, 0, 1, 0, 0)')
      let result = Transform::parser()
        .parse_to_end("matrix(1, 0, 0, 1, 0, 0)")
        .unwrap();

      assert_eq!(result.value.len(), 1);
      if let TransformFunction::Matrix(m) = &result.value[0] {
        assert_eq!(m.a, 1.0);
        assert_eq!(m.b, 0.0);
        assert_eq!(m.c, 0.0);
        assert_eq!(m.d, 1.0);
        assert_eq!(m.tx, 0.0);
        assert_eq!(m.ty, 0.0);
      } else {
        stylex_panic!("Expected Matrix transform function");
      }

      // Test multiple matrices: 'matrix(1, 0, 0, 1, 0, 0) matrix(1, 0, 0.5, 1.5, 0,
      // 0)'
      let result = Transform::parser()
        .parse_to_end("matrix(1, 0, 0, 1, 0, 0) matrix(1, 0, 0.5, 1.5, 0, 0)")
        .unwrap();

      assert_eq!(result.value.len(), 2);
      if let TransformFunction::Matrix(m1) = &result.value[0] {
        assert_eq!(m1.a, 1.0);
        assert_eq!(m1.c, 0.0);
      }
      if let TransformFunction::Matrix(m2) = &result.value[1] {
        assert_eq!(m2.a, 1.0);
        assert_eq!(m2.c, 0.5);
        assert_eq!(m2.d, 1.5);
      }
    }

    #[test]
    fn matrix3d() {
      // Test: Transform.parse.parse('matrix3d(1, 0, 0, 0, 0, 1, 0, 0, 0, 0.5, 1.5, 0,
      // 0, 0, 0, 1)')
      let result = Transform::parser()
        .parse_to_end("matrix3d(1, 0, 0, 0, 0, 1, 0, 0, 0, 0.5, 1.5, 0, 0, 0, 0, 1)")
        .unwrap();

      assert_eq!(result.value.len(), 1);
      if let TransformFunction::Matrix3d(m) = &result.value[0] {
        assert_eq!(m.args.len(), 16);
        assert_eq!(m.args[0], 1.0);
        assert_eq!(m.args[5], 1.0);
        assert_eq!(m.args[9], 0.5);
        assert_eq!(m.args[10], 1.5);
        assert_eq!(m.args[15], 1.0);
      } else {
        stylex_panic!("Expected Matrix3d transform function");
      }
    }

    #[test]
    fn perspective() {
      // Test: Transform.parse.parse('perspective(100px)')
      let result = Transform::parser()
        .parse_to_end("perspective(100px)")
        .unwrap();

      assert_eq!(result.value.len(), 1);
      if let TransformFunction::Perspective(p) = &result.value[0] {
        assert_eq!(p.length.value, 100.0);
        assert_eq!(p.length.unit, "px");
      } else {
        stylex_panic!("Expected Perspective transform function");
      }
    }

    #[test]
    fn rotate() {
      // Test: Transform.parse.parse('rotate(45deg)')
      let result = Transform::parser().parse_to_end("rotate(45deg)").unwrap();

      assert_eq!(result.value.len(), 1);
      if let TransformFunction::Rotate(r) = &result.value[0] {
        assert_eq!(r.angle.value, 45.0);
        assert_eq!(r.angle.unit, "deg");
      } else {
        stylex_panic!("Expected Rotate transform function");
      }
    }

    #[test]
    fn rotate3d() {
      // Test: Transform.parse.parse('rotate3d(1, 2, 3, 45deg)')
      let result = Transform::parser()
        .parse_to_end("rotate3d(1, 2, 3, 45deg)")
        .unwrap();

      assert_eq!(result.value.len(), 1);
      if let TransformFunction::Rotate3d(r) = &result.value[0] {
        assert_eq!(r.x, 1.0);
        assert_eq!(r.y, 2.0);
        assert_eq!(r.z, 3.0);
        assert_eq!(r.angle.value, 45.0);
        assert_eq!(r.angle.unit, "deg");
      } else {
        stylex_panic!("Expected Rotate3d transform function");
      }
    }

    #[test]
    fn rotatex() {
      // Test: Transform.parse.parse('rotateX(45deg)')
      let result = Transform::parser().parse_to_end("rotateX(45deg)").unwrap();

      assert_eq!(result.value.len(), 1);
      if let TransformFunction::RotateXYZ(r) = &result.value[0] {
        assert_eq!(r.angle.value, 45.0);
        assert_eq!(r.angle.unit, "deg");
        assert_eq!(r.axis, Axis::X);
      } else {
        stylex_panic!("Expected RotateXYZ transform function for rotateX");
      }
    }

    #[test]
    fn rotatey() {
      // Test: Transform.parse.parse('rotateY(45deg)')
      let result = Transform::parser().parse_to_end("rotateY(45deg)").unwrap();

      assert_eq!(result.value.len(), 1);
      if let TransformFunction::RotateXYZ(r) = &result.value[0] {
        assert_eq!(r.angle.value, 45.0);
        assert_eq!(r.angle.unit, "deg");
        assert_eq!(r.axis, Axis::Y);
      } else {
        stylex_panic!("Expected RotateXYZ transform function for rotateY");
      }
    }

    #[test]
    fn rotatez() {
      // Test: Transform.parse.parse('rotateZ(45deg)')
      let result = Transform::parser().parse_to_end("rotateZ(45deg)").unwrap();

      assert_eq!(result.value.len(), 1);
      if let TransformFunction::RotateXYZ(r) = &result.value[0] {
        assert_eq!(r.angle.value, 45.0);
        assert_eq!(r.angle.unit, "deg");
        assert_eq!(r.axis, Axis::Z);
      } else {
        stylex_panic!("Expected RotateXYZ transform function for rotateZ");
      }
    }
  }

  #[cfg(test)]
  mod multiple_functions {
    use stylex_macros::stylex_panic;

    use super::*;

    #[test]
    fn perspective_plus_matrix3d() {
      // Test: Transform.parse.parse('perspective(100px)     matrix3d(1, 0, 0, 0, 0,
      // 1, 0, 0, 0, 0.5, 1.5, 0, 0, 0, 0, 1)')
      let result = Transform::parser()
        .parse_to_end(
          "perspective(100px)     matrix3d(1, 0, 0, 0, 0, 1, 0, 0, 0, 0.5, 1.5, 0, 0, 0, 0, 1)",
        )
        .unwrap();

      assert_eq!(result.value.len(), 2);

      // First function: perspective
      if let TransformFunction::Perspective(p) = &result.value[0] {
        assert_eq!(p.length.value, 100.0);
        assert_eq!(p.length.unit, "px");
      } else {
        stylex_panic!("Expected Perspective transform function");
      }

      // Second function: matrix3d
      if let TransformFunction::Matrix3d(m) = &result.value[1] {
        assert_eq!(m.args.len(), 16);
        assert_eq!(m.args[0], 1.0);
        assert_eq!(m.args[9], 0.5);
        assert_eq!(m.args[10], 1.5);
      } else {
        stylex_panic!("Expected Matrix3d transform function");
      }
    }

    #[test]
    fn scale_plus_rotate() {
      // Test: Transform.parse.parse('scale(2) rotate(45deg)')
      let result = Transform::parser()
        .parse_to_end("scale(2) rotate(45deg)")
        .unwrap();

      assert_eq!(result.value.len(), 2);

      // First function: scale
      if let TransformFunction::Scale(s) = &result.value[0] {
        assert_eq!(s.sx, 2.0);
        assert_eq!(s.sy, None);
      } else {
        stylex_panic!("Expected Scale transform function");
      }

      // Second function: rotate
      if let TransformFunction::Rotate(r) = &result.value[1] {
        assert_eq!(r.angle.value, 45.0);
        assert_eq!(r.angle.unit, "deg");
      } else {
        stylex_panic!("Expected Rotate transform function");
      }
    }

    #[test]
    fn scale3d_plus_rotate3d() {
      // Test: Transform.parse.parse('scale3d(2, 3, 4) rotate3d(1, 2, 3, 45deg)')
      let result = Transform::parser()
        .parse_to_end("scale3d(2, 3, 4) rotate3d(1, 2, 3, 45deg)")
        .unwrap();

      assert_eq!(result.value.len(), 2);

      // First function: scale3d
      if let TransformFunction::Scale3d(s) = &result.value[0] {
        assert_eq!(s.sx, 2.0);
        assert_eq!(s.sy, 3.0);
        assert_eq!(s.sz, 4.0);
      } else {
        stylex_panic!("Expected Scale3d transform function");
      }

      // Second function: rotate3d
      if let TransformFunction::Rotate3d(r) = &result.value[1] {
        assert_eq!(r.x, 1.0);
        assert_eq!(r.y, 2.0);
        assert_eq!(r.z, 3.0);
        assert_eq!(r.angle.value, 45.0);
        assert_eq!(r.angle.unit, "deg");
      } else {
        stylex_panic!("Expected Rotate3d transform function");
      }
    }

    #[test]
    fn scale_plus_rotate_plus_translate_plus_skew() {
      // Test: Transform.parse.parse('scale(2) rotate(45deg) translate(100px)
      // skew(45deg)')
      let result = Transform::parser()
        .parse_to_end("scale(2) rotate(45deg) translate(100px) skew(45deg)")
        .unwrap();

      assert_eq!(result.value.len(), 4);

      // First function: scale
      if let TransformFunction::Scale(s) = &result.value[0] {
        assert_eq!(s.sx, 2.0);
        assert_eq!(s.sy, None);
      } else {
        stylex_panic!("Expected Scale transform function");
      }

      // Second function: rotate
      if let TransformFunction::Rotate(r) = &result.value[1] {
        assert_eq!(r.angle.value, 45.0);
        assert_eq!(r.angle.unit, "deg");
      } else {
        stylex_panic!("Expected Rotate transform function");
      }

      // Third function: translate
      if let TransformFunction::Translate(t) = &result.value[2] {
        assert_eq!(t.tx.to_string(), "100px");
        assert_eq!(t.ty, None);
      } else {
        stylex_panic!("Expected Translate transform function");
      }

      // Fourth function: skew
      if let TransformFunction::Skew(sk) = &result.value[3] {
        assert_eq!(sk.ax.value, 45.0);
        assert_eq!(sk.ax.unit, "deg");
        assert_eq!(sk.ay, None);
      } else {
        stylex_panic!("Expected Skew transform function");
      }
    }
  }
}
