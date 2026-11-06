// Source: crates/stylex-js/src/operators.rs

use super::*;

mod evaluate_bin_expr_tests {
  use super::*;

  #[test]
  fn addition() {
    assert_eq!(evaluate_bin_expr(BinaryOp::Add, 5.0, 3.0), 8.0);
  }

  #[test]
  fn subtraction() {
    assert_eq!(evaluate_bin_expr(BinaryOp::Sub, 10.0, 4.0), 6.0);
  }

  #[test]
  fn multiplication() {
    assert_eq!(evaluate_bin_expr(BinaryOp::Mul, 3.0, 7.0), 21.0);
  }

  #[test]
  fn division() {
    assert_eq!(evaluate_bin_expr(BinaryOp::Div, 15.0, 5.0), 3.0);
  }

  #[test]
  fn modulo() {
    assert_eq!(evaluate_bin_expr(BinaryOp::Mod, 17.0, 5.0), 2.0);
  }

  #[test]
  fn exponentiation() {
    assert_eq!(evaluate_bin_expr(BinaryOp::Exp, 2.0, 10.0), 1024.0);
  }

  #[test]
  fn exponentiation_of_negative_zero_and_negative_infinity_keeps_the_sign_of_node() {
    let rows = [
      (f64::NEG_INFINITY, 2.0, f64::INFINITY),
      (f64::NEG_INFINITY, 3.0, f64::NEG_INFINITY),
      (-0.0, -1.0, f64::NEG_INFINITY),
      (-0.0, -2.0, f64::INFINITY),
    ];

    for (base, exponent, expected) in rows {
      assert_eq!(
        evaluate_bin_expr(BinaryOp::Exp, base, exponent).to_bits(),
        expected.to_bits(),
        "{base} ** {exponent}"
      );
    }
  }

  #[test]
  fn addition_of_large_magnitudes() {
    assert_eq!(evaluate_bin_expr(BinaryOp::Add, 1e15, 1e15), 2e15);
  }

  #[test]
  fn division_with_an_inexact_result() {
    let result = evaluate_bin_expr(BinaryOp::Div, 1.0, 3.0);

    assert!(result > 0.333_333_3 && result < 0.333_333_4);
    assert_eq!(result * 3.0, 1.0);
  }

  #[test]
  fn bitwise_or() {
    assert_eq!(evaluate_bin_expr(BinaryOp::BitOr, 5.0, 3.0), 7.0);
  }

  #[test]
  fn bitwise_and() {
    assert_eq!(evaluate_bin_expr(BinaryOp::BitAnd, 5.0, 3.0), 1.0);
  }

  #[test]
  fn bitwise_xor() {
    assert_eq!(evaluate_bin_expr(BinaryOp::BitXor, 5.0, 3.0), 6.0);
  }

  #[test]
  fn left_shift() {
    assert_eq!(evaluate_bin_expr(BinaryOp::LShift, 1.0, 4.0), 16.0);
  }

  #[test]
  fn right_shift() {
    assert_eq!(evaluate_bin_expr(BinaryOp::RShift, 16.0, 2.0), 4.0);
  }

  #[test]
  fn zero_fill_right_shift_positive() {
    assert_eq!(evaluate_bin_expr(BinaryOp::ZeroFillRShift, 16.0, 2.0), 4.0);
  }

  #[test]
  fn zero_fill_right_shift_reads_its_left_side_as_unsigned() {
    assert_eq!(
      evaluate_bin_expr(BinaryOp::ZeroFillRShift, -1.0, 0.0),
      4_294_967_295.0
    );
    assert_eq!(
      evaluate_bin_expr(BinaryOp::ZeroFillRShift, -1.0, 16.0),
      65535.0
    );
    assert_eq!(
      evaluate_bin_expr(BinaryOp::ZeroFillRShift, -9.0, 0.0),
      4_294_967_287.0
    );
    assert_eq!(
      evaluate_bin_expr(BinaryOp::ZeroFillRShift, -8.0, 1.0),
      2_147_483_644.0
    );
  }

  #[test]
  fn right_shift_keeps_the_sign() {
    assert_eq!(evaluate_bin_expr(BinaryOp::RShift, -8.0, 1.0), -4.0);
    assert_eq!(evaluate_bin_expr(BinaryOp::RShift, -1.0, 16.0), -1.0);
  }

  #[test]
  fn the_bitwise_operators_wrap_into_thirty_two_bits() {
    assert_eq!(
      evaluate_bin_expr(BinaryOp::BitOr, 4_294_967_296.0, 0.0),
      0.0
    );
    assert_eq!(
      evaluate_bin_expr(BinaryOp::BitOr, 3_000_000_000.0, 0.0),
      -1_294_967_296.0
    );
    assert_eq!(evaluate_bin_expr(BinaryOp::LShift, 1.0, 32.0), 1.0);
    assert_eq!(evaluate_bin_expr(BinaryOp::LShift, 1.0, 33.0), 2.0);
    assert_eq!(
      evaluate_bin_expr(BinaryOp::LShift, 1.0, 31.0),
      -2_147_483_648.0
    );
    assert_eq!(evaluate_bin_expr(BinaryOp::BitOr, f64::NAN, 0.0), 0.0);
    assert_eq!(evaluate_bin_expr(BinaryOp::BitOr, f64::INFINITY, 0.0), 0.0);
    assert_eq!(
      evaluate_bin_expr(BinaryOp::BitAnd, 1e21, -1.0),
      -559_939_584.0
    );
  }

  #[test]
  fn division_by_zero_yields_infinity() {
    let result = evaluate_bin_expr(BinaryOp::Div, 1.0, 0.0);
    assert!(result.is_infinite() && result.is_sign_positive());
  }

  #[test]
  fn division_negative_by_zero_yields_neg_infinity() {
    let result = evaluate_bin_expr(BinaryOp::Div, -1.0, 0.0);
    assert!(result.is_infinite() && result.is_sign_negative());
  }

  #[test]
  fn zero_divided_by_zero_is_nan() {
    let result = evaluate_bin_expr(BinaryOp::Div, 0.0, 0.0);
    assert!(result.is_nan());
  }

  #[test]
  fn modulo_by_zero_is_nan() {
    let result = evaluate_bin_expr(BinaryOp::Mod, 5.0, 0.0);
    assert!(result.is_nan());
  }

  #[test]
  fn addition_with_negative_numbers() {
    assert_eq!(evaluate_bin_expr(BinaryOp::Add, -3.0, -7.0), -10.0);
  }

  #[test]
  fn subtraction_with_negative_numbers() {
    assert_eq!(evaluate_bin_expr(BinaryOp::Sub, -3.0, -7.0), 4.0);
  }

  #[test]
  fn multiplication_with_negative_numbers() {
    assert_eq!(evaluate_bin_expr(BinaryOp::Mul, -3.0, 7.0), -21.0);
  }

  #[test]
  fn addition_with_zero() {
    assert_eq!(evaluate_bin_expr(BinaryOp::Add, 0.0, 0.0), 0.0);
  }

  #[test]
  fn multiplication_with_zero() {
    assert_eq!(evaluate_bin_expr(BinaryOp::Mul, 42.0, 0.0), 0.0);
  }

  #[test]
  fn exponentiation_zero_power() {
    assert_eq!(evaluate_bin_expr(BinaryOp::Exp, 5.0, 0.0), 1.0);
  }

  #[test]
  fn exponentiation_negative_exponent() {
    assert_eq!(evaluate_bin_expr(BinaryOp::Exp, 2.0, -1.0), 0.5);
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

  #[test]
  fn exponentiation_by_nought_is_one_whatever_the_base() {
    assert_eq!(evaluate_bin_expr(BinaryOp::Exp, f64::NAN, 0.0), 1.0);
    assert_eq!(evaluate_bin_expr(BinaryOp::Exp, f64::INFINITY, 0.0), 1.0);
    assert_eq!(evaluate_bin_expr(BinaryOp::Exp, 2.0, -0.0), 1.0);
  }

  #[test]
  fn exponentiation_keeps_the_rows_the_two_readings_share() {
    assert_eq!(evaluate_bin_expr(BinaryOp::Exp, 2.0, 1024.0), f64::INFINITY);
    assert!(evaluate_bin_expr(BinaryOp::Exp, -8.0, 1.0 / 3.0).is_nan());
    assert_eq!(
      evaluate_bin_expr(BinaryOp::Exp, 2.0, f64::INFINITY),
      f64::INFINITY
    );
    assert_eq!(evaluate_bin_expr(BinaryOp::Exp, 0.5, f64::INFINITY), 0.0);
  }

  // Node uses multiplication for x ** 2; C pow can differ by one bit (evaluator ADR 0008).
  #[test]
  fn exponentiation_by_two_is_one_multiplication() {
    let cases = [
      (730.8721542358398, 534174.1058373372),
      (-730.8721542358398, 534174.1058373372),
      (677916.2883758545, 459570494045.29474),
      (1e200, f64::INFINITY),
    ];
    for (base, expected) in cases {
      assert_eq!(
        evaluate_bin_expr(BinaryOp::Exp, base, 2.0).to_bits(),
        expected.to_bits(),
        "{base} ** 2"
      );
    }
    assert_eq!(evaluate_bin_expr(BinaryOp::Exp, -0.0, 2.0).to_bits(), 0);
    assert!(evaluate_bin_expr(BinaryOp::Exp, f64::NAN, 2.0).is_nan());
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
  fn right_shift_negative_number() {
    assert_eq!(evaluate_bin_expr(BinaryOp::RShift, -16.0, 2.0), -4.0);
  }

  #[test]
  #[should_panic(expected = "Unsupported binary operator")]
  fn unsupported_operator_panics() {
    evaluate_bin_expr(BinaryOp::EqEq, 1.0, 1.0);
  }
}
