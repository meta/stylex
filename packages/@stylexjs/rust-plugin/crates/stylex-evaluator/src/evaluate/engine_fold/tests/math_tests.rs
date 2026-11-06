//! The replaced `Math` statics as the reference compiler folds them.
//!
//! Every expected value is the answer of Node for the same arguments. The rows
//! of the `Math.hypot` table are arguments of similar size, because then the
//! order and the method of the summation change the last bit of the result. A
//! fold of two-argument `hypot` calls gives a different answer for each of
//! these rows.

use std::f64::consts::{FRAC_PI_2, FRAC_PI_6, PI};

use boa_engine::{Context, JsBigInt, JsSymbol, JsValue};
use swc_core::ecma::ast::{Expr, Lit};

use stylex_constants::constants::evaluation_errors::{global_as_a_value, reserved_compiler_name};

use super::{EXPONENTIATE, math_hypot, ported_statics};
use crate::evaluate::source_evaluation::*;

use super::super::engine_reads::{an_engine, answered_by};

/// Arguments and the answer of the reference compiler for them.
const REFERENCE_ROWS: &[(&[f64], f64)] = &[
  (&[2.0, 3.0], 3.6055512754639896),
  (
    &[
      1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0,
      18.0, 19.0, 20.0,
    ],
    53.57238094391549,
  ),
  (&[1e154, 1e154], 1.4142135623730953e154),
  (&[0.1, 0.2], 0.223606797749979),
  (&[1e300, 1e300, 1e300, 1e300], 2e300),
  (&[3e-160, 4e-160], 5e-160),
  (&[5e-324, 5e-324], 5e-324),
  (&[-7.0], 7.0),
  (
    &[
      6.2077e-254,
      6.3e-254,
      4.9608999491e-254,
      -9.706385135650636e-254,
      -1.184732913970947e-254,
    ],
    1.4087355041809873e-253,
  ),
  (
    &[3.18e274, 9.9481111765e274, 4.50159e274],
    1.1372846104124694e275,
  ),
  (
    &[
      3.41476202011108e239,
      4.113137722e239,
      7.71e239,
      -2.331e239,
      -6.0600698e238,
    ],
    9.686248346699622e239,
  ),
  (
    &[7e274, -2.72354e274, -6.6020345687866e274],
    1.0000226526386952e275,
  ),
  (
    &[6.24341488e182, -9.881e182, 8e182, -6.8470424414e182],
    1.5732017688718071e183,
  ),
  (
    &[
      -1.09787225723267e-36,
      4.1927886e-36,
      -3.77414942e-36,
      8.632905483245849e-36,
      4.6061e-36,
      6.194970607757569e-36,
    ],
    1.2928645664013454e-35,
  ),
  (
    &[
      7.857658267021179e-227,
      8.1520986557007e-227,
      -6.5194941e-227,
    ],
    1.3065347651459498e-226,
  ),
  (
    &[
      8.2392621e-26,
      -1.35e-26,
      -5.5055112e-26,
      -9.41611648e-26,
      6.43886e-26,
    ],
    1.5170390937294576e-25,
  ),
  (
    &[
      600000000000000000.0,
      981805146000000000.0,
      -980191230000000000.0,
      678420000000000000.0,
      -453799200000000000.0,
    ],
    1717819432420542500.0,
  ),
  (
    &[-6.3360786438e153, 3.691375255584717e153, -5e153],
    8.875367252004932e153,
  ),
  (
    &[
      -210000000000000000000.0,
      638517439365390000000.0,
      -700000000000000000000.0,
      -124303000000000000000.0,
    ],
    978394478818607500000.0,
  ),
  (&[2e180, -8.954e180, -9.487615e180], 1.3198066312465058e181),
  (
    &[
      -5e296,
      9.25334513e296,
      -4.26575422287e296,
      -9.638e296,
      2e296,
    ],
    1.502371788822021e297,
  ),
];

/// The source of a `Math.hypot` call with `arguments`.
fn hypot_call(arguments: &[f64]) -> String {
  let arguments: Vec<String> = arguments.iter().map(|value| format!("{value:?}")).collect();
  format!("Math.hypot({})", arguments.join(", "))
}

/// The number that `source` folds to, with its sign.
#[track_caller]
fn folded_number(source: &str) -> f64 {
  match assert_folds(source) {
    Expr::Lit(Lit::Num(num)) => num.value,
    other => panic!("expected `{}` to fold to a number, got {:?}", source, other),
  }
}

#[test]
fn math_hypot_folds_to_the_value_of_the_reference_compiler() {
  for (arguments, expected) in REFERENCE_ROWS {
    assert_folds_to_number(&hypot_call(arguments), *expected);
  }
}

#[test]
fn math_hypot_in_a_callback_folds_to_the_value_of_the_reference_compiler() {
  // The engine calls the function from its own code here, not from the fold.
  // The value is the first row of the table.
  assert_folds_to_number(
    "[2, 3].reduce((a, b) => Math.hypot(a, b))",
    3.6055512754639896,
  );
}

#[test]
fn math_hypot_without_arguments_folds_to_zero() {
  assert_folds_to_number("Math.hypot()", 0.0);
}

#[test]
fn math_hypot_of_zeros_folds_to_positive_zero() {
  let folded = folded_number("Math.hypot(-0, -0, 0)");

  assert_eq!(folded, 0.0);
  assert!(
    folded.is_sign_positive(),
    "`Math.hypot(-0, -0, 0)` folded to -0"
  );
}

#[test]
fn math_hypot_prefers_infinity_over_nan_in_any_position() {
  assert_folds_to_number("Math.hypot(NaN, Infinity)", f64::INFINITY);
  assert_folds_to_number("Math.hypot(-Infinity, NaN)", f64::INFINITY);
  assert_folds_to_number("Math.hypot(1, NaN, 2, -Infinity)", f64::INFINITY);
}

#[test]
fn math_hypot_with_nan_and_no_infinity_folds_to_nan() {
  assert_folds_to_nan("Math.hypot(1, NaN)");
  assert_folds_to_nan("Math.hypot(NaN, 0)");
  assert_folds_to_nan("Math.hypot('a', 1)");
}

#[test]
fn math_hypot_overflows_only_when_the_answer_does() {
  assert_folds_to_number(
    "Math.hypot(1.7976931348623157e308, 1.7976931348623157e308)",
    f64::INFINITY,
  );
  assert_folds_to_number("Math.hypot(1e308, 1e308)", 1.4142135623730951e308);
}

#[test]
fn math_hypot_converts_every_argument_to_a_number() {
  assert_folds_to_number("Math.hypot('3', [4], true)", 5.0990195135927845);
  assert_folds_to_number("Math.hypot(null, '', [], 5)", 5.0);
}

#[test]
fn math_hypot_keeps_the_length_and_name_of_the_builtin() {
  let mut engine = an_engine();

  assert_eq!(
    answered_by(&mut engine.context, "() => Math.hypot.length", &[]),
    "2"
  );
  assert_eq!(
    answered_by(&mut engine.context, "() => Math.hypot.name", &[]),
    "hypot"
  );
  assert_eq!(
    answered_by(
      &mut engine.context,
      "() => Object.keys(Math).includes('hypot')",
      &[]
    ),
    "false"
  );
}

/// A symbol at the end of `arguments`, which has no number value.
fn with_a_symbol(mut arguments: Vec<JsValue>) -> Vec<JsValue> {
  let Some(symbol) = JsSymbol::new(None) else {
    panic!("the engine made no symbol");
  };
  arguments.push(JsValue::from(symbol));
  arguments
}

#[track_caller]
fn assert_throws(arguments: &[JsValue], case: &str) {
  let mut context = Context::default();

  assert!(
    math_hypot(&JsValue::undefined(), arguments, &mut context).is_err(),
    "`Math.hypot` did not throw for {case}"
  );
}

#[test]
fn math_hypot_throws_when_an_argument_has_no_number_value() {
  assert_throws(&with_a_symbol(vec![JsValue::from(1)]), "a symbol");
  assert_throws(&[JsValue::from(JsBigInt::from(1))], "a BigInt");
}

#[test]
fn math_hypot_converts_every_argument_before_it_returns() {
  // An infinite argument decides the result, but only after the conversions.
  assert_throws(
    &with_a_symbol(vec![JsValue::from(f64::INFINITY)]),
    "a symbol after Infinity",
  );
  assert_throws(
    &with_a_symbol(vec![JsValue::from(f64::NAN)]),
    "a symbol after NaN",
  );
}

/// The `length` of `Math[name]` in Node. It is not read from the table, so a
/// static put in the table with the wrong arity fails the test.
fn node_length(name: &str) -> usize {
  match name {
    "atan2" | "pow" => 2,
    _ => 1,
  }
}

#[test]
fn every_ported_static_keeps_the_shape_of_the_builtin() {
  let mut engine = an_engine();

  for (name, port) in ported_statics() {
    assert_eq!(
      port.length(),
      node_length(name),
      "the table has the wrong arity for `{name}`"
    );
    // A built-in method is not a constructor and has no `prototype`.
    let read = format!(
      "() => {{ const d = Object.getOwnPropertyDescriptor(Math, '{name}'); \
       let made; try {{ new d.value(); made = 'constructed'; }} \
       catch (e) {{ made = e.constructor.name; }} \
       return [d.value.length, d.value.name, d.writable, d.enumerable, d.configurable, \
       'prototype' in d.value, made].join(); }}"
    );
    assert_eq!(
      answered_by(&mut engine.context, &read, &[]),
      format!(
        "{},{name},true,false,true,false,TypeError",
        node_length(name)
      ),
      "`Math.{name}` does not look like the built-in"
    );
  }
}

#[test]
fn every_ported_static_throws_when_an_argument_has_no_number_value() {
  let mut engine = an_engine();

  for (name, _) in ported_statics() {
    let length = node_length(name);
    for no_number in ["Symbol()", "1n"] {
      for position in 0..length {
        let mut arguments = vec!["1"; length];
        arguments[position] = no_number;
        let call = format!("Math.{name}({})", arguments.join(", "));
        let read = format!(
          "() => {{ try {{ {call}; return 'returned'; }} \
           catch (e) {{ return e.constructor.name; }} }}"
        );
        assert_eq!(
          answered_by(&mut engine.context, &read, &[]),
          "TypeError",
          "`{call}` did not throw a TypeError"
        );
      }
    }
  }
}

#[test]
fn every_ported_static_does_not_convert_an_extra_argument() {
  let mut engine = an_engine();

  for (name, _) in ported_statics() {
    let arguments = vec!["1"; node_length(name)].join(", ");
    let read = format!(
      "() => {{ Math.{name}({arguments}, {{ valueOf() {{ throw 1; }} }}); return 'returned'; }}"
    );
    assert_eq!(
      answered_by(&mut engine.context, &read, &[]),
      "returned",
      "`Math.{name}` converted an argument after its last one"
    );
  }
}

#[test]
fn every_ported_static_reads_a_missing_argument_as_nan() {
  for (name, _) in ported_statics() {
    assert_folds_to_nan(&format!("Math.{name}()"));
    if node_length(name) == 2 {
      assert_folds_to_nan(&format!("Math.{name}(2)"));
    }
  }
}

#[test]
fn math_atan2_converts_y_before_x() {
  let mut engine = an_engine();
  let arguments = "const order = []; \
     const y = { valueOf() { order.push('y'); return 1; } }; \
     const x = { valueOf() { order.push('x'); return 2; } };";

  assert_eq!(
    answered_by(
      &mut engine.context,
      &format!("() => {{ {arguments} Math.atan2(y, x); return order.join(); }}"),
      &[]
    ),
    "y,x"
  );
  // A throw in the first conversion stops the call before the second one.
  assert_eq!(
    answered_by(
      &mut engine.context,
      &format!(
        "() => {{ {arguments} try {{ Math.atan2(Symbol(), x); }} catch {{}} \
         return order.join(); }}"
      ),
      &[]
    ),
    ""
  );
}

#[test]
fn math_asin_folds_to_the_value_of_the_reference_compiler() {
  assert_folds_to_number("Math.asin(0.5)", FRAC_PI_6);
  assert_folds_to_number("Math.asin('-1')", -FRAC_PI_2);
  assert_folds_to_nan("Math.asin(2)");
}

#[test]
fn math_acos_folds_to_the_value_of_the_reference_compiler() {
  assert_folds_to_number("Math.acos(0.05)", 1.5207754699891267);
  assert_folds_to_number("Math.acos(-1)", PI);
  assert_folds_to_nan("Math.acos(-2)");
}

#[test]
fn math_atan_folds_to_the_value_of_the_reference_compiler() {
  assert_folds_to_number("Math.atan(0.5)", 0.4636476090008061);
  assert_folds_to_number("Math.atan(-Infinity)", -FRAC_PI_2);
}

#[test]
fn math_atan2_folds_to_the_value_of_the_reference_compiler() {
  assert_folds_to_number("Math.atan2(0.08, 0.3)", 0.26060239174734096);
  assert_folds_to_number("Math.atan2('1', [-2], 3)", 2.677945044588987);
  assert_folds_to_number("Math.atan2(-Infinity, -Infinity)", -2.356194490192345);
  assert_folds_to_nan("Math.atan2(1, NaN)");
}

#[test]
fn math_cbrt_folds_to_the_value_of_the_reference_compiler() {
  assert_folds_to_number("Math.cbrt(0.02)", 0.27144176165949063);
  assert_folds_to_number("Math.cbrt('-8', 1)", -2.0);
  assert_folds_to_number("Math.cbrt(5e-324)", 1.7031839360032603e-108);
  assert_folds_to_number("Math.cbrt(-Infinity)", f64::NEG_INFINITY);
}

#[test]
fn math_sin_folds_to_the_value_of_the_reference_compiler() {
  assert_folds_to_number("Math.sin(0.51)", 0.48817724688290753);
  assert_folds_to_number("Math.sin('3', 1)", 0.1411200080598672);
  assert_folds_to_number("Math.sin(1e300)", -0.8178819121159085);
  assert_folds_to_nan("Math.sin(Infinity)");
}

#[test]
fn math_cos_folds_to_the_value_of_the_reference_compiler() {
  assert_folds_to_number("Math.cos(0.1)", 0.9950041652780257);
  assert_folds_to_number("Math.cos(-0)", 1.0);
  assert_folds_to_number("Math.cos(1e22)", 0.523214785395139);
  assert_folds_to_nan("Math.cos(-Infinity)");
}

#[test]
fn math_tan_folds_to_the_value_of_the_reference_compiler() {
  assert_folds_to_number("Math.tan(1)", 1.5574077246549023);
  assert_folds_to_number("Math.tan(1.5707963267948966)", 1.633123935319537e16);
  assert_folds_to_number("Math.tan(1e300)", 1.4214488238747245);
  assert_folds_to_nan("Math.tan(NaN)");
}

/// Calls that Node folds to -0. A port that loses the sign of a zero folds
/// them to +0.
const NEGATIVE_ZERO_CALLS: &[&str] = &[
  "Math.asin(-0)",
  "Math.asinh(-0)",
  "Math.atan(-0)",
  "Math.atan2(-0, 1)",
  "Math.atan2(-1, Infinity)",
  "Math.atanh(-0)",
  "Math.cbrt(-0)",
  "Math.expm1(-0)",
  "Math.log1p(-0)",
  "Math.sin(-0)",
  "Math.sinh(-0)",
  "Math.tan(-0)",
  "Math.tanh(-0)",
];

#[test]
fn every_fdlibm_static_folds_to_negative_zero_where_node_does() {
  for &source in NEGATIVE_ZERO_CALLS {
    let folded = folded_number(source);

    assert_eq!(folded, 0.0, "`{source}` did not fold to a zero");
    assert!(folded.is_sign_negative(), "`{source}` folded to +0");
  }
}

#[test]
fn math_exp_folds_to_the_value_of_the_reference_compiler() {
  assert_folds_to_number("Math.exp(0.27)", 1.3099644507332475);
  assert_folds_to_number("Math.exp('1')", std::f64::consts::E);
  assert_folds_to_number("Math.exp(-Infinity)", 0.0);
}

#[test]
fn math_expm1_folds_to_the_value_of_the_reference_compiler() {
  assert_folds_to_number("Math.expm1(1)", 1.718281828459045);
}

#[test]
fn math_log_folds_to_the_value_of_the_reference_compiler() {
  assert_folds_to_number("Math.log(0.09)", -2.407945608651872);
  assert_folds_to_number("Math.log(0)", f64::NEG_INFINITY);
}

#[test]
fn math_log10_folds_to_the_value_of_the_reference_compiler() {
  assert_folds_to_number("Math.log10(0.52)", -0.2839966563652008);
}

#[test]
fn math_log2_folds_to_the_value_of_the_reference_compiler() {
  assert_folds_to_number("Math.log2(1.3)", 0.3785116232537299);
}

#[test]
fn math_log1p_folds_to_the_value_of_the_reference_compiler() {
  assert_folds_to_number("Math.log1p(0.2)", 0.18232155679395462);
}

#[test]
fn math_sinh_folds_to_the_value_of_the_reference_compiler() {
  assert_folds_to_number("Math.sinh(0.2)", 0.20133600254109402);
}

#[test]
fn math_cosh_folds_to_the_value_of_the_reference_compiler() {
  assert_folds_to_number("Math.cosh(0.4)", 1.081072371838455);
}

#[test]
fn math_tanh_folds_to_the_value_of_the_reference_compiler() {
  assert_folds_to_number("Math.tanh(0.02)", 0.01999733375993093);
}

#[test]
fn math_asinh_folds_to_the_value_of_the_reference_compiler() {
  assert_folds_to_number("Math.asinh(0.08)", 0.07991491149449678);
}

#[test]
fn math_acosh_folds_to_the_value_of_the_reference_compiler() {
  assert_folds_to_number("Math.acosh(1.1)", 0.4435682543851154);
}

#[test]
fn math_atanh_folds_to_the_value_of_the_reference_compiler() {
  assert_folds_to_number("Math.atanh(0.5)", 0.5493061443340548);
}

/// Node computes an exponent of 2 as one multiplication and an exponent of one
/// half as a square root. The `pow` of the C library on an arm64 Mac gives a
/// different last bit for each of these arguments.
#[test]
fn math_pow_folds_to_the_value_of_the_reference_compiler() {
  assert_folds_to_number("Math.pow(730.8721542358398, 2)", 534174.1058373372);
  assert_folds_to_number(
    "Math.pow('0.05471760034561157', [0.5])",
    0.23391793506615002,
  );
  assert_folds_to_number("Math.pow(-Infinity, 0.5)", f64::INFINITY);
  assert_folds_to_number("Math.pow(2, 10, 'extra')", 1024.0);
  assert_folds_to_nan("Math.pow(1, Infinity)");
  assert_folds_to_nan("Math.pow(NaN, 2)");
}

#[test]
fn math_pow_of_negative_zero_and_one_half_is_positive_zero() {
  assert_eq!(folded_number("Math.pow(-0, 0.5)").to_bits(), 0);
}

#[test]
fn math_pow_in_a_callback_folds_to_the_value_of_the_reference_compiler() {
  assert_folds_to_number(
    "[730.8721542358398].map((x) => Math.pow(x, 2))[0]",
    534174.1058373372,
  );
}

#[test]
fn math_pow_converts_the_base_before_the_exponent() {
  let mut engine = an_engine();
  let read = "() => { const order = []; \
     Math.pow({ valueOf() { order.push('base'); return 2; } }, \
     { valueOf() { order.push('exponent'); return 3; } }); return order.join(); }";

  assert_eq!(answered_by(&mut engine.context, read, &[]), "base,exponent");
}

// ==================== `**` in the source that the engine runs ====================

/// `**` as the engine folds it, with the answer of Node for each row. Boa
/// computes a number to an integer power with repeated multiplication, and the
/// other rows with the `pow` of the C library. Each row folded to a different
/// last bit, or to a different value, before `**` used the evaluator's steps.
const EXPONENTIATION_ROWS: &[(&str, &str)] = &[
  ("String(1.092492 ** 15)", "3.7694152359253774"),
  ("String(1.1 ** -9)", "0.4240976183724846"),
  ("String(952.4673882682695 ** 0.5)", "30.86207038207692"),
  ("String(3 ** 40)", "12157665459056929000"),
  ("String(7 ** 30)", "2.2539340290692256e+25"),
  ("String(1 ** NaN)", "NaN"),
  ("String(2 ** 3 ** 2)", "512"),
  ("String((-0) ** 0.5)", "0"),
  ("String((-Infinity) ** 0.5)", "Infinity"),
  ("String((-0) ** -1)", "-Infinity"),
  ("String((-Infinity) ** 3)", "-Infinity"),
  ("String('2' ** [3])", "8"),
];

#[test]
fn exponentiation_in_the_engine_folds_to_the_value_of_the_reference_compiler() {
  for (source, expected) in EXPONENTIATION_ROWS {
    assert_folds_to_string(source, expected);
  }
}

#[test]
fn exponentiation_in_a_callback_folds_to_the_value_of_the_reference_compiler() {
  assert_folds_to_strings(
    "[15, -9].map((n) => String(1.092492 ** n))",
    &["3.7694152359253774", "0.45106134545857757"],
  );
  assert_folds_to_nan("[NaN].map((x) => 1 ** x)[0]");
}

#[test]
fn exponentiation_in_a_callback_body_folds_to_the_value_of_the_reference_compiler() {
  assert_folds_to_string(
    "[1.092492].map((x) => { const y = x ** 15; return String(y); })[0]",
    "3.7694152359253774",
  );
}

/// Powers whose value the language fixes, or which are exact, as
/// `(base, exponent, value)`. Every correct `pow` gives these values, so they
/// do not depend on the host or on the version of Node.
const LANGUAGE_FIXED_POWERS: &[(&str, &str, f64)] = &[
  // A zero exponent gives 1 for every base, and a NaN exponent gives NaN for
  // every other base. A base of magnitude 1 under an infinite exponent gives
  // NaN, where the `pow` of the C library gives 1.
  ("NaN", "0", 1.0),
  ("NaN", "-0", 1.0),
  ("NaN", "1", f64::NAN),
  ("1", "NaN", f64::NAN),
  ("1", "Infinity", f64::NAN),
  ("-1", "Infinity", f64::NAN),
  ("-1", "-Infinity", f64::NAN),
  // Infinite exponents and bases.
  ("2", "Infinity", f64::INFINITY),
  ("0.5", "Infinity", 0.0),
  ("2", "-Infinity", 0.0),
  ("0.5", "-Infinity", f64::INFINITY),
  ("Infinity", "-1", 0.0),
  ("Infinity", "0.5", f64::INFINITY),
  ("-Infinity", "3", f64::NEG_INFINITY),
  ("-Infinity", "2", f64::INFINITY),
  ("-Infinity", "-3", -0.0),
  ("-Infinity", "0.5", f64::INFINITY),
  // A zero base. The sign of the result comes from the parity of the exponent.
  ("0", "-1", f64::INFINITY),
  ("-0", "-3", f64::NEG_INFINITY),
  ("-0", "-2", f64::INFINITY),
  ("-0", "3", -0.0),
  ("-0", "0.5", 0.0),
  // A negative base: NaN for an exponent that is not an integer.
  ("-8", "1 / 3", f64::NAN),
  ("-4", "0.5", f64::NAN),
  ("-2", "3", -8.0),
  ("-2", "-1", -0.5),
  ("-1", "9007199254740991", -1.0),
  ("-1", "1e300", 1.0),
  // The ends of the range of a number: the largest power of 2, the smallest
  // subnormal number, and a result past each end.
  ("2", "53", 9_007_199_254_740_992.0),
  ("2", "1023", 8.98846567431158e307),
  ("2", "-1074", 5e-324),
  ("2", "1024", f64::INFINITY),
  ("2", "-1075", 0.0),
  ("-10", "309", f64::NEG_INFINITY),
  ("2", "1e10", f64::INFINITY),
  ("0.5", "1e10", 0.0),
  // Operands that are not numbers are converted to numbers first.
  ("'2'", "'3'", 8.0),
  ("true", "2", 1.0),
  ("null", "0", 1.0),
  ("undefined", "1", f64::NAN),
  ("'abc'", "2", f64::NAN),
];

/// The spellings of `base ** exponent` that reach each fold of a power: the
/// evaluator, the native `__sxPow` that the engine calls for `**`, and the
/// native `Math.pow`.
const POW_SPELLINGS: &[fn(&str, &str) -> String] = &[
  |base, exponent| format!("({base}) ** ({exponent})"),
  |base, exponent| format!("[{base}].map((x) => x ** ({exponent}))[0]"),
  |base, exponent| format!("Math.pow({base}, {exponent})"),
];

#[test]
fn every_fold_of_a_power_gives_the_value_that_the_language_fixes() {
  for spell in POW_SPELLINGS {
    for &(base, exponent, expected) in LANGUAGE_FIXED_POWERS {
      let source = spell(base, exponent);
      let folded = folded_number(&source);

      // The bits compare the sign of a zero too. NaN has more than one bit
      // pattern, so it is compared by kind.
      if expected.is_nan() {
        assert!(folded.is_nan(), "`{source}` folded to {folded}, not NaN");
      } else {
        assert_eq!(
          folded.to_bits(),
          expected.to_bits(),
          "`{source}` folded to {folded}"
        );
      }
    }
  }
}

#[test]
fn exponentiation_in_the_evaluator_is_right_associative() {
  let rows = [
    ("2 ** 3 ** 2", 512.0),
    ("(2 ** 3) ** 2", 64.0),
    ("(-2) ** 3 ** 2", -512.0),
    ("2 ** 2 ** 2 ** 2", 65_536.0),
    ("2 ** 2 ** 2 ** 2 ** 2", f64::INFINITY),
  ];

  for (source, expected) in rows {
    assert_folds_to_number(source, expected);
  }
}

/// A power nested 15 levels deep, below the depth ceiling. The exponents are 2
/// and one half, so the value is the same on every host. Node gives it too.
#[test]
fn a_nested_power_folds_level_by_level() {
  let source = (0..15).fold(String::from("1.5"), |source, level| {
    format!("({source}) ** {}", if level % 2 == 0 { "0.5" } else { "2" })
  });

  assert_folds_to_number(&source, 1.224744871391589);
}

/// A BigInt has no number, so `**` over one refuses in each fold. The
/// reference implementation refuses each row too.
#[test]
fn exponentiation_of_a_big_integer_refuses() {
  for source in [
    "2n ** 3n",
    "String(2n ** 64n)",
    "[2n].map((x) => x ** 2n)[0]",
  ] {
    assert_deopts(source);
  }
}

#[test]
fn an_exponentiation_assignment_is_not_folded_by_the_engine() {
  // The guard admits no assignment, so `**=` never reaches the engine.
  assert_deopts("[2].map((x) => { x **= 2; return x; })");
}

#[test]
fn the_engine_exponentiates_bigints_as_the_language_does() {
  let mut engine = an_engine();
  let rows = [
    (format!("{EXPONENTIATE}(2n, 3n)"), "8"),
    (
      format!("{EXPONENTIATE}({{ valueOf() {{ return 3n; }} }}, 2n)"),
      "9",
    ),
  ];

  for (call, expected) in rows {
    assert_eq!(
      answered_by(&mut engine.context, &format!("() => String({call})"), &[]),
      expected,
      "`{call}` did not answer as `**` does"
    );
  }
}

#[test]
fn the_engine_refuses_to_mix_a_bigint_and_a_number_in_an_exponentiation() {
  let mut engine = an_engine();
  let rows = [
    ("2n, 3", "TypeError"),
    ("2, 3n", "TypeError"),
    ("2n, -1n", "RangeError"),
  ];

  for (operands, expected) in rows {
    let read = format!(
      "() => {{ try {{ {EXPONENTIATE}({operands}); return 'returned'; }} \
       catch (e) {{ return e.constructor.name; }} }}"
    );

    assert_eq!(
      answered_by(&mut engine.context, &read, &[]),
      expected,
      "`{EXPONENTIATE}({operands})` did not throw as `**` does"
    );
  }
}

#[test]
fn the_engine_converts_the_base_and_the_exponent_in_order() {
  let mut engine = an_engine();
  let read = format!(
    "() => {{ const order = []; \
     {EXPONENTIATE}({{ valueOf() {{ order.push('base'); return 2; }} }}, \
     {{ valueOf() {{ order.push('exponent'); return 3; }} }}); return order.join(); }}"
  );

  assert_eq!(
    answered_by(&mut engine.context, &read, &[]),
    "base,exponent"
  );
}

#[test]
fn the_engine_stops_an_exponentiation_at_the_first_conversion_that_throws() {
  let mut engine = an_engine();
  // Each operand records its conversion, so the answer shows which operands
  // converted before the throw. The thrown value comes out unchanged.
  let base = "{ valueOf() { order.push('base'); return 2; } }";
  let exponent = "{ valueOf() { order.push('exponent'); return 3; } }";
  let throws = "{ valueOf() { order.push('throw'); throw 'thrown'; } }";
  let rows = [
    (format!("Symbol(), {exponent}"), "TypeError|"),
    (format!("{throws}, {exponent}"), "thrown|throw"),
    (format!("{base}, Symbol()"), "TypeError|base"),
    (format!("{base}, {throws}"), "thrown|base,throw"),
    (format!("{throws}, {throws}"), "thrown|throw"),
    (format!("1n, {throws}"), "thrown|throw"),
  ];

  for (operands, expected) in rows {
    let read = format!(
      "() => {{ const order = []; let caught; \
       try {{ {EXPONENTIATE}({operands}); caught = 'returned'; }} \
       catch (e) {{ caught = typeof e === 'string' ? e : e.constructor.name; }} \
       return caught + '|' + order.join(); }}"
    );

    assert_eq!(
      answered_by(&mut engine.context, &read, &[]),
      expected,
      "`{EXPONENTIATE}({operands})` did not stop at the first throw"
    );
  }
}

#[test]
fn the_exponentiation_native_reads_two_operands_and_no_more() {
  let mut engine = an_engine();
  // A missing operand is `undefined`, which converts to `NaN`. A third one is
  // never converted, so its throw does not happen.
  let rows = [
    (format!("{EXPONENTIATE}()"), "NaN"),
    (format!("{EXPONENTIATE}(2)"), "NaN"),
    (format!("{EXPONENTIATE}(undefined, 0)"), "1"),
    (
      format!("{EXPONENTIATE}(2, 3, {{ valueOf() {{ throw 1; }} }})"),
      "8",
    ),
  ];

  for (call, expected) in rows {
    assert_eq!(
      answered_by(&mut engine.context, &format!("() => String({call})"), &[]),
      expected,
      "`{call}` did not answer as `**` does"
    );
  }
}

#[test]
fn an_exponentiation_whose_operand_throws_refuses_before_the_engine() {
  // The guard refuses the object method, so a conversion that throws never
  // gets to the engine from a fold. Only the engine tests above reach it.
  for source in [
    "[{ valueOf() { throw 1; } }].map((x) => x ** 2)",
    "[{ valueOf() { throw 1; } }].map((x) => 2 ** x)",
  ] {
    assert_deopt_reason_contains(source, "Unsupported object method");
  }
}

/// The count of `**` in each long chain, far past the default depth ceiling.
const LONG_CHAIN: usize = 500;

/// A ceiling that admits every chain of [`LONG_CHAIN`] steps.
const LONG_CHAIN_CEILING: usize = 4 * LONG_CHAIN;

/// `x ** 1 ** ... ** 1 ** 3` in a callback, with `steps` operators. `**`
/// groups from the right, so the ones above the base make an exponent of one.
fn right_chain(steps: usize) -> String {
  format!(
    "[2].map((x) => String(x ** {}3))[0]",
    "1 ** ".repeat(steps - 1)
  )
}

/// `((x ** 2) ** 0.5) ...` in a callback, with `steps` operators. Node squares
/// and roots 2 exactly, so the value comes back to 2 after every pair.
fn left_chain(steps: usize) -> String {
  format!(
    "[2].map((x) => String({}x{}))[0]",
    "(".repeat(steps),
    ") ** 2) ** 0.5".repeat(steps / 2)
  )
}

#[test]
fn a_long_exponentiation_chain_in_a_callback_folds_in_the_order_of_the_language() {
  // The engine runs the callback, so each `**` of the chain becomes a call of
  // the native, nested as deep as the chain.
  assert_folds_to_string_with_ceiling(&right_chain(LONG_CHAIN), "2", LONG_CHAIN_CEILING);
  assert_folds_to_string_with_ceiling(&left_chain(LONG_CHAIN), "2", LONG_CHAIN_CEILING);
}

#[test]
fn a_long_exponentiation_chain_refuses_at_the_default_depth_ceiling() {
  for chain in [
    right_chain(LONG_CHAIN),
    left_chain(LONG_CHAIN),
    format!("String(2 ** {}3)", "1 ** ".repeat(LONG_CHAIN)),
  ] {
    assert_deopt_reason_contains(&chain, "too deeply nested");
  }
}

#[test]
fn the_exponentiation_native_cannot_be_replaced_or_listed() {
  let mut engine = an_engine();
  let read = format!(
    "() => {{ const d = Object.getOwnPropertyDescriptor(globalThis, '{EXPONENTIATE}'); \
     return [d.writable, d.enumerable, d.configurable, d.value.length].join(); }}"
  );

  assert_eq!(
    answered_by(&mut engine.context, &read, &[]),
    "false,false,false,2"
  );
}

#[test]
fn a_name_that_spells_the_exponentiation_native_refuses() {
  assert_deopt_reason_contains(
    &format!("[2].map(({EXPONENTIATE}) => {EXPONENTIATE} ** 2)"),
    &reserved_compiler_name(EXPONENTIATE),
  );
  assert_refused_in_a_module_binding(
    EXPONENTIATE,
    "2",
    &format!("[{EXPONENTIATE}].map(String)[0]"),
    &reserved_compiler_name(EXPONENTIATE),
  );
}

#[test]
fn exponentiation_in_a_function_the_module_declares_folds_to_the_value_of_the_reference_compiler() {
  // The function crosses as the source of its declaration, and that source
  // runs in the engine as well.
  assert_eq!(
    folded_in_a_module_binding(
      "power",
      "(x) => x ** 15",
      "String([1.092492].map(power)[0])"
    ),
    "3.7694152359253774"
  );
}

#[test]
fn a_static_handed_to_a_callback_as_a_value_refuses() {
  // The reference compiler deopts here too: `Math` is a global, and a global
  // read as a value has no value at compile time.
  for source in ["[1].map(Math.atan2)[0]", "[0.51].map(Math.sin)[0]"] {
    assert_deopt_reason_contains(source, &global_as_a_value("Math"));
  }
}
