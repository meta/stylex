use crate::coercions::{to_int32, to_shift_count, to_uint32};
use stylex_macros::stylex_panic;
use swc_core::ecma::ast::BinaryOp;

/// Applies a numeric binary operator; panics on unsupported operators.
pub fn evaluate_bin_expr(op: BinaryOp, left: f64, right: f64) -> f64 {
  match &op {
    BinaryOp::Add => left + right,
    BinaryOp::Sub => left - right,
    BinaryOp::Div => left / right,
    BinaryOp::Mul => left * right,
    BinaryOp::Mod => left % right,
    BinaryOp::Exp => js_exponentiate(left, right),
    BinaryOp::BitOr => f64::from(to_int32(left) | to_int32(right)),
    BinaryOp::BitXor => f64::from(to_int32(left) ^ to_int32(right)),
    BinaryOp::BitAnd => f64::from(to_int32(left) & to_int32(right)),
    BinaryOp::LShift => f64::from(to_int32(left) << to_shift_count(right)),
    BinaryOp::RShift => f64::from(to_int32(left) >> to_shift_count(right)),
    BinaryOp::ZeroFillRShift => f64::from(to_uint32(left) >> to_shift_count(right)),
    _ => stylex_panic!("Unsupported binary operator: {:?}", op),
  }
}

/// JavaScript exponentiation with Node's square and square-root fast paths.
/// See tests/operators_tests.rs and evaluator ADR 0008 for Node rounding differences.
pub fn js_exponentiate(base: f64, exponent: f64) -> f64 {
  if exponent == 0.0 {
    return 1.0;
  }

  if exponent.is_nan() || (base.abs() == 1.0 && exponent.is_infinite()) {
    return f64::NAN;
  }

  if exponent == 2.0 {
    return base * base;
  }

  if exponent == 0.5 {
    // Node preserves pow(-Infinity, 0.5) and clears negative zero; see tests/operators_tests.rs.
    return if base == f64::NEG_INFINITY {
      f64::INFINITY
    } else {
      (base + 0.0).sqrt()
    };
  }

  base.powf(exponent)
}

#[cfg(test)]
#[path = "tests/operators_tests.rs"]
mod tests;
