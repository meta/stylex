use crate::coercions::{to_int32, to_shift_count, to_uint32};
use stylex_macros::stylex_panic;
use swc_core::ecma::ast::BinaryOp;

/// Applies a binary operator to two numeric operands.
///
/// Only the operators that produce a number are handled. A comparison or a
/// logical operator reaches a different reader, so one arriving here is a bug
/// in the caller rather than in the source being compiled.
pub fn evaluate_bin_expr(op: BinaryOp, left: f64, right: f64) -> f64 {
  match &op {
    BinaryOp::Add => left + right,
    BinaryOp::Sub => left - right,
    BinaryOp::Div => left / right,
    BinaryOp::Mul => left * right,
    BinaryOp::Mod => left % right,
    BinaryOp::Exp => js_exponentiate(left, right),
    // Every bitwise operator reads its sides through `ToInt32` and its count
    // through `ToShiftCount`, which is what makes them 32-bit: a 64-bit cast
    // saturates where the language wraps, and it answered `0` for `-1 >>> 0`
    // where JavaScript answers 4294967295.
    BinaryOp::BitOr => f64::from(to_int32(left) | to_int32(right)),
    BinaryOp::BitXor => f64::from(to_int32(left) ^ to_int32(right)),
    BinaryOp::BitAnd => f64::from(to_int32(left) & to_int32(right)),
    BinaryOp::LShift => f64::from(to_int32(left) << to_shift_count(right)),
    BinaryOp::RShift => f64::from(to_int32(left) >> to_shift_count(right)),
    BinaryOp::ZeroFillRShift => f64::from(to_uint32(left) >> to_shift_count(right)),
    _ => stylex_panic!("Unsupported binary operator: {:?}", op),
  }
}

/// `**` and `Math.pow`, with the steps of Node. The evaluator calls it for
/// `**`, and the engine fold calls it for `**` and for `Math.pow`.
///
/// The result differs from IEEE `pow` on the three rows the language names.
/// `pow` answers `1` for a base of 1 whatever the exponent, `NaN` included, and
/// for a base of magnitude 1 under an infinite exponent. The language answers
/// `NaN` for all three. The zero exponent is read first, because both readings
/// answer `1` for it whatever the base, `NaN` included.
///
/// Node computes an exponent of 2 as one multiplication and an exponent of one
/// half as a square root. Both round correctly, so they give the same bits on
/// every host. The `pow` of the C library can round them differently in the
/// last bit. For every other exponent, Node 24 also calls the `pow` of the C
/// library, so its answer depends on the host. ADR 0008 of the evaluator says
/// which answer this function gives on Node 22.
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
    // The square root of -Infinity is NaN, but `pow` gives Infinity. The `+ 0`
    // changes -0 to +0, as `pow` does.
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
