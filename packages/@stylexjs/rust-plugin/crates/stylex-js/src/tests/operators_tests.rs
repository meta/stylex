// Source: crates/stylex-js/src/operators.rs

use super::*;

mod evaluate_bin_expr_tests {
  use super::*;

  #[test]
  fn exponentiation_zero_power() {
    assert_eq!(evaluate_bin_expr(BinaryOp::Exp, 5.0, 0.0), 1.0);
  }

  #[test]
  fn exponentiation_answers_not_a_number_where_the_language_does() {
    let cases: [(f64, f64); 4] = [
      (1.0, f64::NAN),
      (-1.0, f64::INFINITY),
      (1.0, f64::INFINITY),
      (-1.0, f64::NEG_INFINITY),
    ];

    for (base, exponent) in cases {
      assert!(
        evaluate_bin_expr(BinaryOp::Exp, base, exponent).is_nan(),
        "{} ** {}",
        base,
        exponent
      );
    }
  }

  // Node uses sqrt(x + 0), except -Infinity yields Infinity; see operators.rs::js_exponentiate.
  #[test]
  fn exponentiation_by_one_half_is_the_square_root() {
    let cases = [
      (0.05471760034561157, 0.23391793506615002),
      (3.6914396286010742e-301, 6.075721873655075e-151),
      (5e-324, 2.2227587494850775e-162),
      (f64::NEG_INFINITY, f64::INFINITY),
      (-0.0, 0.0),
    ];
    for (base, expected) in cases {
      assert_eq!(
        evaluate_bin_expr(BinaryOp::Exp, base, 0.5).to_bits(),
        expected.to_bits(),
        "{base} ** 0.5"
      );
    }
    assert!(evaluate_bin_expr(BinaryOp::Exp, -4.0, 0.5).is_nan());
    assert!(evaluate_bin_expr(BinaryOp::Exp, f64::NAN, 0.5).is_nan());
  }

  #[test]
  #[should_panic(expected = "Unsupported binary operator")]
  fn unsupported_operator_panics() {
    evaluate_bin_expr(BinaryOp::EqEq, 1.0, 1.0);
  }
}
