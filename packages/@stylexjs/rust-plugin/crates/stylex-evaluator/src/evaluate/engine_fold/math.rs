//! The `Math` statics whose engine answer is not the answer of the reference
//! compiler. ADR 0008 says when to add one.
//!
//! The reference compiler folds a `Math` call with the `Math` of Node. The
//! engine here has its own `Math`, and most of it gives the same numbers. Some
//! statics do not:
//!
//! - `Math.hypot` folds two arguments at a time through the `hypot` of the C
//!   library. Node scales every argument by the largest one and adds the
//!   squares with a compensated sum.
//! - The other statics in [`FDLIBM_STATICS`] call the C library. Node calls its
//!   own copy of fdlibm. The ports give the answer of Node on x64 on every
//!   host (see [`fdlibm`]).
//! - `Math.pow` uses the `pow` of the C library for every exponent. Node uses
//!   one multiplication for an exponent of 2 and a square root for an exponent
//!   of one half. [`MATH_POW`] does the steps of `**` in the evaluator.
//! - `**` is not a static, but it has the same problem. Boa multiplies again
//!   and again for an integer exponent, and it answers `1` for `1 ** NaN`.
//!
//! The two methods can differ in the last bit, and a different number makes a
//! different class name. The C library also differs between platforms, so the
//! engine's answer did too.
//!
//! [`install`] replaces each of these statics with a native function that does
//! the steps that Node does, in the same order. It also adds the native
//! [`EXPONENTIATE`], and [`exponentiation_as_native_calls`] changes each `**`
//! of the printed source into a call of it.

mod fdlibm;

use boa_engine::{
  Context, JsArgs, JsObject, JsResult, JsString, JsValue, NativeFunction,
  object::FunctionObjectBuilder, property::PropertyDescriptor, value::Numeric,
};
use swc_core::{
  common::util::take::Take,
  ecma::{
    ast::{BinExpr, BinaryOp, Expr, ExprOrSpread},
    visit::{VisitMut, VisitMutWith},
  },
};

use stylex_ast::ast::factories::create_ident_call_expr;
use stylex_js::operators::js_exponentiate;

/// The global name of the native that the printed source calls for `**`.
///
/// It is a global and not a parameter of the printed arrow, so a fold with
/// `**` needs no arrow and no extra argument. Nothing can change what the name
/// holds. The guard refuses every name of this spelling, so no binding can
/// shadow it, and the property cannot be written or deleted.
pub(super) const EXPONENTIATE: &str = "__sxPow";

/// The `length` of the native [`EXPONENTIATE`]: a base and an exponent.
const EXPONENTIATE_LENGTH: usize = 2;

/// The `length` of `Math.hypot`: the two arguments that its definition names.
const MATH_HYPOT_LENGTH: usize = 2;

/// The native function that replaces a static, by its count of arguments.
#[derive(Clone, Copy)]
enum Port {
  /// A static of one argument.
  Unary(fn(f64) -> f64),
  /// A static of two arguments, in the order of the JavaScript call.
  Binary(fn(f64, f64) -> f64),
}

impl Port {
  /// The `length` of the static: the count of its arguments.
  const fn length(self) -> usize {
    match self {
      Port::Unary(_) => 1,
      Port::Binary(_) => 2,
    }
  }
}

/// The statics that Node computes with fdlibm, and the port of each one.
/// `fdlibmStatics` in the addon spec `mathStatics.spec.ts` names the same
/// statics of one argument, and the spec has one test for each static of two
/// arguments. Change the table and the spec together.
const FDLIBM_STATICS: &[(&str, Port)] = &[
  ("acos", Port::Unary(fdlibm::acos)),
  ("acosh", Port::Unary(fdlibm::acosh)),
  ("asin", Port::Unary(fdlibm::asin)),
  ("asinh", Port::Unary(fdlibm::asinh)),
  ("atan", Port::Unary(fdlibm::atan)),
  ("atan2", Port::Binary(fdlibm::atan2)),
  ("atanh", Port::Unary(fdlibm::atanh)),
  ("cbrt", Port::Unary(fdlibm::cbrt)),
  ("cos", Port::Unary(fdlibm::cos)),
  ("cosh", Port::Unary(fdlibm::cosh)),
  ("exp", Port::Unary(fdlibm::exp)),
  ("expm1", Port::Unary(fdlibm::expm1)),
  ("log", Port::Unary(fdlibm::log)),
  ("log10", Port::Unary(fdlibm::log10)),
  ("log1p", Port::Unary(fdlibm::log1p)),
  ("log2", Port::Unary(fdlibm::log2)),
  ("sin", Port::Unary(fdlibm::sin)),
  ("sinh", Port::Unary(fdlibm::sinh)),
  ("tan", Port::Unary(fdlibm::tan)),
  ("tanh", Port::Unary(fdlibm::tanh)),
];

/// `Math.pow`, which converts its arguments as `atan2` does and computes as
/// `**` does.
const MATH_POW: (&str, Port) = ("pow", Port::Binary(js_exponentiate));

/// Every static that [`install`] replaces through a [`Port`]: the statics in
/// [`FDLIBM_STATICS`], and `Math.pow`.
fn ported_statics() -> impl Iterator<Item = (&'static str, Port)> {
  FDLIBM_STATICS.iter().copied().chain([MATH_POW])
}

/// Replaces `Math.hypot` and every static of [`ported_statics`] in `context`,
/// and adds the native [`EXPONENTIATE`] to its global object.
pub(super) fn install(context: &mut Context) {
  let exponentiate = native_function(
    context,
    EXPONENTIATE,
    EXPONENTIATE_LENGTH,
    NativeFunction::from_fn_ptr(exponentiate),
  );

  // Not writable, not enumerable and not configurable, so the source that the
  // engine runs cannot replace it, delete it or find it in a list of globals.
  // The insert cannot fail, so its result is not read.
  context.global_object().insert_property(
    JsString::from(EXPONENTIATE),
    PropertyDescriptor::builder()
      .value(exponentiate)
      .writable(false)
      .enumerable(false)
      .configurable(false),
  );

  insert_method(
    context,
    "hypot",
    MATH_HYPOT_LENGTH,
    NativeFunction::from_fn_ptr(math_hypot),
  );

  for (name, port) in ported_statics() {
    insert_method(
      context,
      name,
      port.length(),
      NativeFunction::from_copy_closure(move |_, arguments, context| {
        // The arguments convert in order, so the first one that throws stops
        // the call. A missing argument is `undefined`, which converts to `NaN`.
        let result = match port {
          Port::Unary(function) => function(arguments.get_or_undefined(0).to_number(context)?),
          Port::Binary(function) => {
            let first = arguments.get_or_undefined(0).to_number(context)?;
            let second = arguments.get_or_undefined(1).to_number(context)?;
            function(first, second)
          },
        };
        Ok(JsValue::from(result))
      }),
    );
  }
}

/// Puts `method` on the `Math` of `context` as `name`, in place of the old one.
///
/// The property gets the attributes of every built-in method: writable,
/// configurable and not enumerable. The insert does not check the old property
/// and cannot fail. It returns `true` when it replaces a property, which is
/// always true here, so the result is not read.
fn insert_method(context: &mut Context, name: &str, length: usize, method: NativeFunction) {
  let function = native_function(context, name, length, method);

  context.intrinsics().objects().math().insert_property(
    JsString::from(name),
    PropertyDescriptor::builder()
      .value(function)
      .writable(true)
      .enumerable(false)
      .configurable(true),
  );
}

/// `method` as a function object with the `name` and the `length` of a
/// built-in. Like a built-in, it is not a constructor and has no `prototype`.
fn native_function(
  context: &mut Context,
  name: &str,
  length: usize,
  method: NativeFunction,
) -> JsObject {
  FunctionObjectBuilder::new(context.realm(), method)
    .name(JsString::from(name))
    .length(length)
    .build()
    .into()
}

/// `**`, with the steps of the language and the rounding of Node.
///
/// Both operands convert to a numeric value in order, as `**` converts them.
/// Two numbers go to [`js_exponentiate`]. Any other pair has a BigInt, and the
/// engine's own `**` answers it: a BigInt result, or the `TypeError` for a
/// BigInt with a number. The operands are primitive at that point, so they do
/// not convert a second time.
fn exponentiate(_: &JsValue, arguments: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
  let base = arguments.get_or_undefined(0).to_numeric(context)?;
  let exponent = arguments.get_or_undefined(1).to_numeric(context)?;

  match (base, exponent) {
    (Numeric::Number(base), Numeric::Number(exponent)) => {
      Ok(JsValue::from(js_exponentiate(base, exponent)))
    },
    (base, exponent) => JsValue::from(base).pow(&JsValue::from(exponent), context),
  }
}

/// Changes each `a ** b` in `expr` into a call of the native [`EXPONENTIATE`].
///
/// A call evaluates its callee, then `a`, then `b`, and then converts both, so
/// the order of `**` does not change. The guard admits no assignment, so no
/// `**=` gets to the engine and this has nothing to do for one.
///
/// The fold memo keeps the printed source, so this runs once for each printed
/// shape, not once for each fold.
pub(super) fn exponentiation_as_native_calls(expr: &mut Expr) {
  expr.visit_mut_with(&mut ExponentiationRewrite);
}

/// The walk of [`exponentiation_as_native_calls`].
///
/// Bottom-up, so `a ** b ** c` becomes a call whose second argument is the
/// call for `b ** c`.
struct ExponentiationRewrite;

impl VisitMut for ExponentiationRewrite {
  fn visit_mut_expr(&mut self, expr: &mut Expr) {
    expr.visit_mut_children_with(self);

    if let Expr::Bin(BinExpr {
      op: BinaryOp::Exp,
      left,
      right,
      ..
    }) = expr
    {
      let operands = [left, right].map(|operand| ExprOrSpread {
        spread: None,
        expr: operand.take(),
      });

      *expr = Expr::Call(create_ident_call_expr(EXPONENTIATE, operands.into()));
    }
  }
}

/// `Math.hypot`, with the steps and the rounding of Node.
///
/// It converts every argument to a number before it looks at any of them, so
/// a conversion that throws throws before the result is known. An infinite
/// argument wins over `NaN`. Each value is divided by the largest one before
/// it is squared, so no square overflows or underflows. The squares are added
/// with Kahan summation, which keeps the rounding error of each addition and
/// subtracts it from the next one.
fn math_hypot(_: &JsValue, arguments: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
  let length = arguments.len();
  if length == 0 {
    return Ok(JsValue::from(0));
  }

  // A NaN argument keeps its zero here; the result is decided before the sum.
  let mut abs_values = vec![0.0_f64; length];
  let mut one_arg_is_nan = false;
  let mut max = 0.0_f64;
  for (argument, abs_value) in arguments.iter().zip(abs_values.iter_mut()) {
    let value = argument.to_number(context)?;
    if value.is_nan() {
      one_arg_is_nan = true;
    } else {
      *abs_value = value.abs();
      if *abs_value > max {
        max = *abs_value;
      }
    }
  }

  if max == f64::INFINITY {
    return Ok(JsValue::from(f64::INFINITY));
  } else if one_arg_is_nan {
    return Ok(JsValue::from(f64::NAN));
  } else if max == 0.0 {
    return Ok(JsValue::from(0));
  }

  let mut sum = 0.0_f64;
  let mut compensation = 0.0_f64;
  for abs_value in abs_values {
    let n = abs_value / max;
    let summand = n * n - compensation;
    let preliminary = sum + summand;
    compensation = (preliminary - sum) - summand;
    sum = preliminary;
  }

  Ok(JsValue::from(sum.sqrt() * max))
}

#[cfg(test)]
#[path = "tests/math_tests.rs"]
mod math_tests;
