// Tests for the ECMAScript coercions.
// Source: crates/stylex-js/src/coercions.rs
//
// Expected values are what `String(x)` and `Number(x)` answer in a JavaScript
// runtime, which is what `@stylexjs/babel-plugin` folds those calls to.

use super::*;
use stylex_utils::string::utf16_length;
use swc_core::{
  atoms::{
    Wtf8Atom,
    wtf8::{CodePoint, Wtf8Buf},
  },
  common::DUMMY_SP,
  ecma::ast::{
    ArrayLit, ArrowExpr, ArrowFunctionBody, AssignProp, BigInt, BindingIdent, Bool,
    ComputedPropName, ExprOrSpread, Function, FunctionBody, GetterProp, Ident, IdentName,
    KeyValueProp, MethodProp, Null, Number, ObjectLit, ParenExpr, Pat, Prop, PropName,
    PropOrSpread, Regex, SetterProp, SpreadElement, Str, ThisExpr, UnaryExpr,
  },
};

fn str_expr(value: &str) -> Expr {
  Expr::Lit(Lit::Str(Str {
    span: DUMMY_SP,
    value: value.into(),
    raw: None,
  }))
}

fn num_expr(value: f64) -> Expr {
  Expr::Lit(Lit::Num(Number {
    span: DUMMY_SP,
    value,
    raw: None,
  }))
}

fn bool_expr(value: bool) -> Expr {
  Expr::Lit(Lit::Bool(Bool {
    span: DUMMY_SP,
    value,
  }))
}

fn null_expr() -> Expr {
  Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))
}

fn ident_expr(name: &str) -> Expr {
  Expr::Ident(Ident::new(name.into(), DUMMY_SP, Default::default()))
}

fn ident(name: &str) -> Ident {
  Ident::new(name.into(), DUMMY_SP, Default::default())
}

fn big_int_expr(value: i64) -> Expr {
  Expr::Lit(Lit::BigInt(BigInt {
    span: DUMMY_SP,
    value: Box::new(value.into()),
    raw: None,
  }))
}

fn regex_expr(exp: &str, flags: &str) -> Expr {
  Expr::Lit(Lit::Regex(Regex {
    span: DUMMY_SP,
    exp: exp.into(),
    flags: flags.into(),
  }))
}

fn array_expr(elems: Vec<Option<Expr>>) -> Expr {
  Expr::Array(ArrayLit {
    span: DUMMY_SP,
    elems: elems
      .into_iter()
      .map(|elem| {
        elem.map(|expr| ExprOrSpread {
          spread: None,
          expr: Box::new(expr),
        })
      })
      .collect(),
  })
}

fn spread_array_expr(inner: Expr) -> Expr {
  Expr::Array(ArrayLit {
    span: DUMMY_SP,
    elems: vec![Some(ExprOrSpread {
      spread: Some(DUMMY_SP),
      expr: Box::new(inner),
    })],
  })
}

fn empty_object_expr() -> Expr {
  Expr::Object(ObjectLit {
    span: DUMMY_SP,
    props: vec![],
  })
}

fn arrow_expr() -> Expr {
  Expr::Arrow(ArrowExpr {
    span: DUMMY_SP,
    params: vec![],
    body: Box::new(ArrowFunctionBody::Expr(Box::new(num_expr(1.0)))),
    is_async: false,
    is_generator: false,
    type_params: None,
    return_type: None,
    ctxt: Default::default(),
  })
}

#[test]
fn strings_are_their_own_coercion() {
  assert_eq!(to_js_string(&str_expr("#fff")).as_deref(), Some("#fff"));
  assert_eq!(to_js_string(&str_expr("")).as_deref(), Some(""));
  assert_eq!(to_js_string(&str_expr("null")).as_deref(), Some("null"));
}

#[test]
fn numbers_use_the_javascript_spelling() {
  assert_eq!(to_js_string(&num_expr(1.0)).as_deref(), Some("1"));
  assert_eq!(to_js_string(&num_expr(-0.0)).as_deref(), Some("0"));
  assert_eq!(to_js_string(&num_expr(1.5)).as_deref(), Some("1.5"));
  // Rust's `Display` would spell these out in full; JavaScript does not.
  assert_eq!(to_js_string(&num_expr(1e21)).as_deref(), Some("1e+21"));
  assert_eq!(to_js_string(&num_expr(1e-7)).as_deref(), Some("1e-7"));
  assert_eq!(to_js_string(&num_expr(f64::NAN)).as_deref(), Some("NaN"));
  assert_eq!(
    to_js_string(&num_expr(f64::INFINITY)).as_deref(),
    Some("Infinity")
  );
}

#[test]
fn booleans_null_and_undefined_use_their_javascript_spellings() {
  assert_eq!(to_js_string(&bool_expr(true)).as_deref(), Some("true"));
  assert_eq!(to_js_string(&bool_expr(false)).as_deref(), Some("false"));
  assert_eq!(to_js_string(&null_expr()).as_deref(), Some("null"));
  assert_eq!(
    to_js_string(&ident_expr("undefined")).as_deref(),
    Some("undefined")
  );
  assert_eq!(to_js_string(&ident_expr("NaN")).as_deref(), Some("NaN"));
  assert_eq!(
    to_js_string(&ident_expr("Infinity")).as_deref(),
    Some("Infinity")
  );
}

#[test]
fn a_big_integer_renders_its_digits_without_the_suffix() {
  assert_eq!(to_js_string(&big_int_expr(1)).as_deref(), Some("1"));
  assert_eq!(to_js_string(&big_int_expr(-42)).as_deref(), Some("-42"));
}

#[test]
fn a_regular_expression_renders_its_own_source() {
  assert_eq!(to_js_string(&regex_expr("x", "")).as_deref(), Some("/x/"));
  assert_eq!(
    to_js_string(&regex_expr("a+b", "gi")).as_deref(),
    Some("/a+b/gi")
  );
  // Its source is not a numeric literal, so it has a number all the same.
  assert!(
    to_js_number(&regex_expr("x", ""))
      .expect("a regular expression has a number")
      .is_nan()
  );
}

#[test]
fn an_identifier_that_is_not_a_global_value_has_no_coercion() {
  // Anything else would have been inlined from its binding before reaching
  // here, so an identifier that survives is not a value.
  assert_eq!(to_js_string(&ident_expr("someBinding")), None);
}

#[test]
fn arrays_join_with_commas() {
  assert_eq!(to_js_string(&array_expr(vec![])).as_deref(), Some(""));
  assert_eq!(
    to_js_string(&array_expr(vec![Some(str_expr("a")), Some(str_expr("b"))])).as_deref(),
    Some("a,b")
  );
  assert_eq!(
    to_js_string(&array_expr(vec![Some(num_expr(1.0))])).as_deref(),
    Some("1")
  );
}

#[test]
fn array_elements_with_no_value_join_as_nothing() {
  assert_eq!(
    to_js_string(&array_expr(vec![
      Some(null_expr()),
      Some(ident_expr("undefined")),
      Some(num_expr(1.0)),
    ]))
    .as_deref(),
    Some(",,1")
  );
  // A hole, which is neither `null` nor `undefined` but joins the same way.
  assert_eq!(
    to_js_string(&array_expr(vec![
      Some(num_expr(1.0)),
      None,
      Some(num_expr(2.0))
    ]))
    .as_deref(),
    Some("1,,2")
  );
}

#[test]
fn nested_arrays_flatten_through_the_join() {
  assert_eq!(
    to_js_string(&array_expr(vec![
      Some(num_expr(1.0)),
      Some(array_expr(vec![Some(num_expr(2.0)), Some(num_expr(3.0))])),
    ]))
    .as_deref(),
    Some("1,2,3")
  );
}

#[test]
fn an_unevaluated_spread_element_has_no_coercion() {
  assert_eq!(to_js_string(&spread_array_expr(array_expr(vec![]))), None);
}

#[test]
fn objects_use_the_object_prototype_default() {
  assert_eq!(
    to_js_string(&empty_object_expr()).as_deref(),
    Some(OBJECT_TO_STRING)
  );

  let with_prop = Expr::Object(ObjectLit {
    span: DUMMY_SP,
    props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
      key: PropName::Ident(IdentName::new("a".into(), DUMMY_SP)),
      value: Box::new(num_expr(1.0)),
    })))],
  });

  assert_eq!(to_js_string(&with_prop).as_deref(), Some("[object Object]"));
}

fn object_expr(props: Vec<PropOrSpread>) -> Expr {
  Expr::Object(ObjectLit {
    span: DUMMY_SP,
    props,
  })
}

fn key_value_prop(key: PropName, value: Expr) -> PropOrSpread {
  PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
    key,
    value: Box::new(value),
  })))
}

fn ident_key(name: &str) -> PropName {
  PropName::Ident(IdentName::new(name.into(), DUMMY_SP))
}

/// A zero-parameter arrow returning `body` — the shape an own conversion
/// method is written in.
fn returning_arrow(body: Expr) -> Expr {
  Expr::Arrow(ArrowExpr {
    span: DUMMY_SP,
    params: vec![],
    body: Box::new(ArrowFunctionBody::Expr(Box::new(body))),
    is_async: false,
    is_generator: false,
    type_params: None,
    return_type: None,
    ctxt: Default::default(),
  })
}

#[test]
fn an_own_to_string_answers_the_string_coercion() {
  // `String({ toString: () => 'red' })` is `'red'` in a runtime, so answering
  // the `Object.prototype` default would be a value no runtime produces.
  let overriding = object_expr(vec![key_value_prop(
    ident_key("toString"),
    returning_arrow(str_expr("red")),
  )]);

  assert_eq!(to_js_string(&overriding).as_deref(), Some("red"));

  // The same name spelled as a string key is the same own property.
  let string_key = object_expr(vec![key_value_prop(
    PropName::Str(Str {
      span: DUMMY_SP,
      value: "toString".into(),
      raw: None,
    }),
    returning_arrow(str_expr("red")),
  )]);

  assert_eq!(to_js_string(&string_key).as_deref(), Some("red"));
}

#[test]
fn the_two_hints_ask_for_the_methods_in_opposite_orders() {
  // `String` prefers `toString` and `Number` prefers `valueOf`, which is the
  // whole of the difference between them.
  let both = object_expr(vec![
    key_value_prop(ident_key("toString"), returning_arrow(str_expr("1"))),
    key_value_prop(ident_key("valueOf"), returning_arrow(num_expr(2.0))),
  ]);

  assert_eq!(to_js_string(&both).as_deref(), Some("1"));
  assert_eq!(to_js_number(&both), Some(2.0));
}

#[test]
fn a_missing_method_falls_through_the_way_object_prototype_does() {
  // `Object.prototype.valueOf` answers the object itself, which is not a
  // primitive, so a number falls through to an own `toString`.
  let to_string_only = object_expr(vec![key_value_prop(
    ident_key("toString"),
    returning_arrow(str_expr("7")),
  )]);

  assert_eq!(to_js_number(&to_string_only), Some(7.0));

  // `Object.prototype.toString` answers a primitive, so a string never reaches
  // an own `valueOf`.
  let value_of_only = object_expr(vec![key_value_prop(
    ident_key("valueOf"),
    returning_arrow(str_expr("v")),
  )]);

  assert_eq!(
    to_js_string(&value_of_only).as_deref(),
    Some(OBJECT_TO_STRING)
  );
  // The number still goes through that own `valueOf`, whose `'v'` is not a
  // numeric literal.
  assert!(to_js_number(&value_of_only).is_some_and(f64::is_nan));
}

#[test]
fn a_method_in_a_form_that_cannot_be_applied_has_no_coercion() {
  // A value that is not callable ends in a `TypeError` rather than a value,
  // and a method answering an object has not answered a primitive.
  let not_callable = object_expr(vec![key_value_prop(
    ident_key("toString"),
    str_expr("notfn"),
  )]);

  assert_eq!(to_js_string(&not_callable), None);

  let returns_object = object_expr(vec![key_value_prop(
    ident_key("toString"),
    returning_arrow(empty_object_expr()),
  )]);

  assert_eq!(to_js_string(&returns_object), None);
}

#[test]
fn an_object_whose_keys_cannot_be_named_has_no_coercion() {
  // A spread may carry an override in, and a computed key is how
  // `Symbol.toPrimitive` is spelled — neither can be read off the literal.
  let spread = object_expr(vec![PropOrSpread::Spread(SpreadElement {
    dot3_token: DUMMY_SP,
    expr: Box::new(empty_object_expr()),
  })]);

  assert_eq!(to_js_string(&spread), None);

  let computed = object_expr(vec![key_value_prop(
    PropName::Computed(ComputedPropName {
      span: DUMMY_SP,
      expr: Box::new(str_expr("a")),
    }),
    num_expr(1.0),
  )]);

  assert_eq!(to_js_string(&computed), None);
}

#[test]
fn an_object_carrying_the_override_deeper_still_coerces() {
  // Only an *own* key replaces the default: a nested one does not.
  let nested = object_expr(vec![key_value_prop(
    ident_key("a"),
    object_expr(vec![key_value_prop(ident_key("toString"), arrow_expr())]),
  )]);

  assert_eq!(to_js_string(&nested).as_deref(), Some(OBJECT_TO_STRING));
}

#[test]
fn an_overriding_object_inside_an_array_converts_through_its_own_method() {
  // `Array.prototype.join` takes each element's `ToString`, own method and
  // all.
  let element = object_expr(vec![key_value_prop(
    ident_key("toString"),
    returning_arrow(str_expr("z")),
  )]);

  assert_eq!(
    to_js_string(&array_expr(vec![Some(num_expr(1.0)), Some(element)])).as_deref(),
    Some("1,z")
  );
}

#[test]
fn a_function_has_no_compile_time_coercion() {
  // `String(fn)` is the function's source text, which the evaluator does not
  // retain — so there is no answer to give.
  assert_eq!(to_js_string(&arrow_expr()), None);
}

#[test]
fn only_null_and_undefined_join_as_empty() {
  assert!(joins_as_empty(&null_expr()));
  assert!(joins_as_empty(&ident_expr("undefined")));
  assert!(!joins_as_empty(&str_expr("")));
  assert!(!joins_as_empty(&num_expr(0.0)));
  assert!(!joins_as_empty(&ident_expr("NaN")));
}

#[test]
fn a_string_of_decimal_digits_is_its_value() {
  assert_eq!(string_to_js_number("10"), 10.0);
  assert_eq!(string_to_js_number("00.5"), 0.5);
  assert_eq!(string_to_js_number(".5"), 0.5);
  assert_eq!(string_to_js_number("5."), 5.0);
  assert_eq!(string_to_js_number("+5"), 5.0);
  assert_eq!(string_to_js_number("-5"), -5.0);
  assert_eq!(string_to_js_number("1e3"), 1000.0);
  assert_eq!(string_to_js_number("1E3"), 1000.0);
  assert_eq!(string_to_js_number("1e-3"), 0.001);
  assert_eq!(string_to_js_number("1e+3"), 1000.0);
}

#[test]
fn an_empty_or_blank_string_is_zero() {
  assert_eq!(string_to_js_number(""), 0.0);
  assert_eq!(string_to_js_number("   "), 0.0);
  assert_eq!(
    string_to_js_number("\t\n\r\u{000B}\u{000C}\u{00A0}\u{FEFF}"),
    0.0
  );
}

#[test]
fn surrounding_whitespace_is_ignored() {
  assert_eq!(string_to_js_number("  10  "), 10.0);
  assert_eq!(string_to_js_number("\t\n\r 7  \u{FEFF}"), 7.0);
  assert_eq!(string_to_js_number("\u{2028}1\u{2029}"), 1.0);
  assert_eq!(string_to_js_number("\u{3000}1\u{2000}"), 1.0);
  // Interior whitespace is not, because it is not part of the literal.
  assert!(string_to_js_number("1 0").is_nan());
  // U+0085 is Unicode whitespace but not JavaScript whitespace, so a string
  // padded with it is not a numeric literal at all.
  assert!(string_to_js_number("\u{0085}1").is_nan());
}

#[test]
fn the_radix_prefixes_are_read_as_javascript_reads_them() {
  assert_eq!(string_to_js_number("0x1f"), 31.0);
  assert_eq!(string_to_js_number("0X1F"), 31.0);
  assert_eq!(string_to_js_number("0b101"), 5.0);
  assert_eq!(string_to_js_number("0B101"), 5.0);
  assert_eq!(string_to_js_number("0o17"), 15.0);
  assert_eq!(string_to_js_number("0O17"), 15.0);
  assert_eq!(string_to_js_number("  0x1f  "), 31.0);
  // Wider than `u128`, so the digits are accumulated rather than converted.
  assert_eq!(
    string_to_js_number("0xdeadbeefdeadbeefdeadbeef"),
    6.891571802158121e28
  );
  assert_eq!(
    string_to_js_number("0x1000000000000000000000000000000000"),
    5.444517870735016e39
  );
}

#[test]
fn a_radix_prefix_without_digits_of_that_radix_is_not_a_number() {
  assert!(string_to_js_number("0x").is_nan());
  assert!(string_to_js_number("0xg").is_nan());
  assert!(string_to_js_number("0b2").is_nan());
  assert!(string_to_js_number("0o8").is_nan());
  // A radix literal takes no sign, and is not a decimal literal either.
  assert!(string_to_js_number("-0x1f").is_nan());
  assert!(string_to_js_number("+0x1f").is_nan());
}

#[test]
fn infinity_is_spelled_the_way_javascript_spells_it() {
  assert_eq!(string_to_js_number("Infinity"), f64::INFINITY);
  assert_eq!(string_to_js_number("+Infinity"), f64::INFINITY);
  assert_eq!(string_to_js_number("-Infinity"), f64::NEG_INFINITY);
  assert_eq!(string_to_js_number("  Infinity  "), f64::INFINITY);
  assert!(string_to_js_number("infinity").is_nan());
  assert!(string_to_js_number("Infinity1").is_nan());
}

#[test]
fn the_rust_float_spellings_are_not_numbers() {
  // Rust's own float parsing accepts all of these; JavaScript accepts none of
  // them, and taking Rust's answer would put `inf` in a stylesheet.
  assert!(string_to_js_number("inf").is_nan());
  assert!(string_to_js_number("-inf").is_nan());
  assert!(string_to_js_number("nan").is_nan());
  assert!(string_to_js_number("NaN").is_nan());
}

#[test]
fn a_string_that_is_not_a_numeric_literal_is_not_a_number() {
  assert!(string_to_js_number("10px").is_nan());
  assert!(string_to_js_number("1_000").is_nan());
  assert!(string_to_js_number("1e").is_nan());
  assert!(string_to_js_number(".").is_nan());
  assert!(string_to_js_number("-").is_nan());
  assert!(string_to_js_number("1e5e5").is_nan());
}

#[test]
fn negative_zero_survives_the_coercion() {
  // It renders as `0`, but it is not `0`, and losing the sign here would lose
  // it everywhere downstream.
  assert!(string_to_js_number("-0").is_sign_negative());
  assert_eq!(string_to_js_number("-0"), 0.0);
  assert!(string_to_js_number("-0.0").is_sign_negative());
  assert!(!string_to_js_number("0").is_sign_negative());
}

#[test]
fn numbers_are_their_own_coercion() {
  assert_eq!(to_js_number(&num_expr(1.5)), Some(1.5));
  assert_eq!(to_js_number(&num_expr(0.0)), Some(0.0));
  assert_eq!(to_js_number(&num_expr(f64::INFINITY)), Some(f64::INFINITY));
  assert!(matches!(to_js_number(&num_expr(f64::NAN)), Some(value) if value.is_nan()));
}

#[test]
fn booleans_null_and_undefined_coerce_to_their_javascript_numbers() {
  assert_eq!(to_js_number(&bool_expr(true)), Some(1.0));
  assert_eq!(to_js_number(&bool_expr(false)), Some(0.0));
  // `null` is zero; `undefined` is not. The two differ here where they agree
  // under `ToString`.
  assert_eq!(to_js_number(&null_expr()), Some(0.0));
  assert!(matches!(to_js_number(&ident_expr("undefined")), Some(value) if value.is_nan()));
  assert!(matches!(to_js_number(&ident_expr("NaN")), Some(value) if value.is_nan()));
  assert_eq!(to_js_number(&ident_expr("Infinity")), Some(f64::INFINITY));
}

#[test]
fn strings_coerce_through_the_numeric_literal_grammar() {
  assert_eq!(to_js_number(&str_expr("0x1f")), Some(31.0));
  assert_eq!(to_js_number(&str_expr("  10  ")), Some(10.0));
  assert_eq!(to_js_number(&str_expr("")), Some(0.0));
  assert!(matches!(to_js_number(&str_expr("10px")), Some(value) if value.is_nan()));
}

#[test]
fn arrays_and_objects_coerce_through_their_string_form() {
  // An array's primitive value is its join, so a one-element array coerces
  // its element and a longer one cannot be a number.
  assert_eq!(to_js_number(&array_expr(vec![])), Some(0.0));
  assert_eq!(
    to_js_number(&array_expr(vec![Some(num_expr(5.0))])),
    Some(5.0)
  );
  assert_eq!(
    to_js_number(&array_expr(vec![Some(str_expr("0x1f"))])),
    Some(31.0)
  );
  assert_eq!(
    to_js_number(&array_expr(vec![Some(null_expr())])),
    Some(0.0)
  );
  assert_eq!(
    to_js_number(&array_expr(vec![Some(array_expr(vec![]))])),
    Some(0.0)
  );
  assert!(matches!(
    to_js_number(&array_expr(vec![Some(num_expr(1.0)), Some(num_expr(2.0))])),
    Some(value) if value.is_nan()
  ));
  // `[object Object]` is not a numeric literal.
  assert!(matches!(to_js_number(&empty_object_expr()), Some(value) if value.is_nan()));
}

#[test]
fn a_function_coerces_to_not_a_number() {
  // Its source text is unknown but irrelevant: no source text is a numeric
  // literal, so the number is `NaN` either way. `ToString` has no such luck.
  assert!(matches!(to_js_number(&arrow_expr()), Some(value) if value.is_nan()));
  // Which means a function inside an array does not make the array's number
  // unknowable either — the join is not a numeric literal whatever it holds.
  assert!(matches!(
    to_js_number(&array_expr(vec![Some(arrow_expr())])),
    Some(value) if value.is_nan()
  ));
  assert!(matches!(
    to_js_number(&array_expr(vec![Some(arrow_expr()), Some(num_expr(2.0))])),
    Some(value) if value.is_nan()
  ));
}

#[test]
fn a_value_with_no_string_form_has_no_number_either() {
  assert_eq!(to_js_number(&ident_expr("someBinding")), None);
  assert_eq!(to_js_number(&spread_array_expr(array_expr(vec![]))), None);
}

#[test]
fn every_value_but_a_function_coerces_to_a_plain_object() {
  // The nullish pair takes a fresh object, an object and an array already are
  // one, and a primitive is boxed in one -- three outcomes `typeof` cannot tell
  // apart, and the one caller left is `typeof`.
  assert_eq!(to_object(&null_expr()), Some(ObjectCoercion::Object));
  assert_eq!(
    to_object(&ident_expr("undefined")),
    Some(ObjectCoercion::Object)
  );
  assert_eq!(
    to_object(&empty_object_expr()),
    Some(ObjectCoercion::Object)
  );
  assert_eq!(
    to_object(&array_expr(vec![Some(str_expr("a"))])),
    Some(ObjectCoercion::Object)
  );
  assert_eq!(to_object(&str_expr("red")), Some(ObjectCoercion::Object));
  assert_eq!(to_object(&num_expr(10.0)), Some(ObjectCoercion::Object));
  assert_eq!(to_object(&bool_expr(true)), Some(ObjectCoercion::Object));
  // The numeric globals arrive as identifiers and box like the numbers they
  // are, as does the `undefined` that arrives the same way.
  assert_eq!(to_object(&ident_expr("NaN")), Some(ObjectCoercion::Object));
  assert_eq!(
    to_object(&ident_expr("Infinity")),
    Some(ObjectCoercion::Object)
  );
}

#[test]
fn a_function_coerces_to_an_object_that_says_it_is_a_function() {
  // An object like the rest, and reported apart because it is the one whose
  // `typeof` is not `object`.
  assert_eq!(to_object(&arrow_expr()), Some(ObjectCoercion::Function));
}

/// An expression the evaluator reduced to no value at all — neither a literal,
/// nor one of the surviving globals, nor an object, array, or function.
fn this_expr() -> Expr {
  Expr::This(ThisExpr { span: DUMMY_SP })
}

fn binding_pat(name: &str) -> Pat {
  Pat::Ident(BindingIdent {
    id: Ident::new(name.into(), DUMMY_SP, Default::default()),
    type_ann: None,
  })
}

/// A one-parameter arrow returning `body` — a form the crate refuses, because a
/// default initialiser on that parameter would make the body depend on it.
fn parameterised_arrow(body: Expr) -> Expr {
  Expr::Arrow(ArrowExpr {
    span: DUMMY_SP,
    params: vec![binding_pat("x")],
    body: Box::new(ArrowFunctionBody::Expr(Box::new(body))),
    is_async: false,
    is_generator: false,
    type_params: None,
    return_type: None,
    ctxt: Default::default(),
  })
}

/// A block-bodied arrow, whose statements are more than the crate reads.
fn block_bodied_arrow() -> Expr {
  Expr::Arrow(ArrowExpr {
    span: DUMMY_SP,
    params: vec![],
    body: Box::new(ArrowFunctionBody::FunctionBody(FunctionBody::default())),
    is_async: false,
    is_generator: false,
    type_params: None,
    return_type: None,
    ctxt: Default::default(),
  })
}

fn getter_prop(name: &str) -> PropOrSpread {
  PropOrSpread::Prop(Box::new(Prop::Getter(GetterProp {
    key: ident_key(name),
    ..Default::default()
  })))
}

fn setter_prop(name: &str) -> PropOrSpread {
  PropOrSpread::Prop(Box::new(Prop::Setter(SetterProp {
    key: ident_key(name),
    function: Box::new(Function {
      params: vec![binding_pat("value").into()],
      ..Default::default()
    }),
    ..Default::default()
  })))
}

fn method_prop(name: &str) -> PropOrSpread {
  PropOrSpread::Prop(Box::new(Prop::Method(MethodProp {
    key: ident_key(name),
    function: Box::default(),
  })))
}

fn shorthand_prop(name: &str) -> PropOrSpread {
  PropOrSpread::Prop(Box::new(Prop::Shorthand(Ident::new(
    name.into(),
    DUMMY_SP,
    Default::default(),
  ))))
}

fn assign_prop(name: &str, value: Expr) -> PropOrSpread {
  PropOrSpread::Prop(Box::new(Prop::Assign(AssignProp {
    span: DUMMY_SP,
    key: Ident::new(name.into(), DUMMY_SP, Default::default()),
    value: Box::new(value),
  })))
}

#[test]
fn an_expression_that_is_not_a_value_has_no_coercion_at_all() {
  // Unlike an identifier, which may still be one of the globals that survive
  // evaluation, this is not a value of any kind — so neither coercion has an
  // answer and its `ToObject` outcome cannot be read off it either.
  assert_eq!(to_js_string(&this_expr()), None);
  assert_eq!(to_js_number(&this_expr()), None);
  assert_eq!(to_object(&this_expr()), None);
}

#[test]
fn an_object_whose_own_method_cannot_be_applied_has_no_number_either() {
  // A number asks for `valueOf` first, so a form that this crate cannot apply
  // refuses before the `Object.prototype` default is ever reached.
  let unapplicable = object_expr(vec![method_prop("valueOf")]);

  assert_eq!(to_js_number(&unapplicable), None);
}

#[test]
fn a_conversion_method_written_as_a_method_or_an_accessor_has_no_coercion() {
  // Each of these is an own `toString`, so the `Object.prototype` default no
  // longer applies — and none of them is a form this crate can apply.
  assert_eq!(
    to_js_string(&object_expr(vec![method_prop("toString")])),
    None
  );
  assert_eq!(
    to_js_string(&object_expr(vec![getter_prop("toString")])),
    None
  );
  assert_eq!(
    to_js_string(&object_expr(vec![setter_prop("toString")])),
    None
  );
}

#[test]
fn a_conversion_arrow_the_crate_cannot_read_has_no_coercion() {
  let parameterised = object_expr(vec![key_value_prop(
    ident_key("toString"),
    parameterised_arrow(str_expr("red")),
  )]);

  assert_eq!(to_js_string(&parameterised), None);

  let block_bodied = object_expr(vec![key_value_prop(
    ident_key("toString"),
    block_bodied_arrow(),
  )]);

  assert_eq!(to_js_string(&block_bodied), None);
}

#[test]
fn a_shorthand_or_assignment_property_names_a_method_the_same_way() {
  // Neither carries a key node, so both are named from the identifier they
  // were written as — and neither is a form the coercion can apply.
  assert_eq!(
    to_js_string(&object_expr(vec![shorthand_prop("toString")])),
    None
  );
  assert_eq!(
    to_js_string(&object_expr(vec![assign_prop("toString", str_expr("red"))])),
    None
  );
  // Under any other name they leave the default in place.
  assert_eq!(
    to_js_string(&object_expr(vec![shorthand_prop("a")])).as_deref(),
    Some(OBJECT_TO_STRING)
  );
}

#[test]
fn a_key_that_is_not_a_name_is_not_a_conversion_method() {
  // A numeric key is readable — it is not the computed key `Symbol.toPrimitive`
  // is spelled with — but it names neither method, so the default stands.
  let numeric = object_expr(vec![key_value_prop(
    PropName::Num(Number {
      span: DUMMY_SP,
      value: 1.0,
      raw: None,
    }),
    str_expr("a"),
  )]);

  assert_eq!(to_js_string(&numeric).as_deref(), Some(OBJECT_TO_STRING));

  let big_int_key = object_expr(vec![key_value_prop(
    PropName::BigInt(BigInt {
      span: DUMMY_SP,
      value: Box::new(1.into()),
      raw: None,
    }),
    str_expr("a"),
  )]);

  assert_eq!(
    to_js_string(&big_int_key).as_deref(),
    Some(OBJECT_TO_STRING)
  );
}

/// The three numbers the grammar has no literal for carry their authored text,
/// and every other number carries none.
///
/// The text is what a reader diffs, what the reference implementation prints, and
/// what a class name is a hash of, so a `Number` node holding `NaN` must not
/// reach the emitter bare — it writes `0 / 0` for one and a numeral no author
/// wrote for an infinity.
#[test]
fn only_the_unspellable_numbers_carry_their_text() {
  let raw_of = |value: f64| match js_number_expr(value) {
    Expr::Lit(Lit::Num(number)) => number.raw.map(|raw| raw.to_string()),
    other => panic!("expected a number, got {:?}", other),
  };

  assert_eq!(raw_of(f64::NAN).as_deref(), Some("NaN"));
  assert_eq!(raw_of(f64::INFINITY).as_deref(), Some("Infinity"));
  assert_eq!(raw_of(f64::NEG_INFINITY).as_deref(), Some("-Infinity"));

  assert_eq!(raw_of(0.0), None);
  assert_eq!(raw_of(-0.0), None);
  assert_eq!(raw_of(1.5), None);
  assert_eq!(raw_of(f64::MAX), None);
}

/// The value is the number itself whatever its text, so a consumer that reads
/// the value rather than the spelling is unaffected by the arm above.
#[test]
fn an_unspellable_number_still_holds_its_value() {
  let value_of = |value: f64| match js_number_expr(value) {
    Expr::Lit(Lit::Num(number)) => number.value,
    other => panic!("expected a number, got {:?}", other),
  };

  assert!(value_of(f64::NAN).is_nan());
  assert_eq!(value_of(f64::INFINITY), f64::INFINITY);
  assert_eq!(value_of(f64::NEG_INFINITY), f64::NEG_INFINITY);
  assert_eq!(value_of(1.5), 1.5);
}

#[test]
fn a_value_of_no_readable_kind_has_no_object_coercion() {
  // Whether this is a function cannot be read off it, so the caller deopts
  // rather than picking an answer.
  assert_eq!(to_object(&ident_expr("someBinding")), None);
  // An array is an object however its elements were written, so a spread does
  // not make its kind unreadable the way it makes its string form unknowable.
  assert_eq!(
    to_object(&spread_array_expr(array_expr(vec![]))),
    Some(ObjectCoercion::Object)
  );
}

/// `void 0` — the third spelling of `undefined`, and the only one that is an
/// operator rather than a literal or a name.
fn void_expr(operand: Expr) -> Expr {
  Expr::Unary(UnaryExpr {
    span: DUMMY_SP,
    op: UnaryOp::Void,
    arg: Box::new(operand),
  })
}

#[test]
fn the_falsy_primitives_are_the_whole_of_the_falsy_list() {
  assert_eq!(to_js_boolean(&str_expr("")), Some(false));
  assert_eq!(to_js_boolean(&num_expr(0.0)), Some(false));
  assert_eq!(to_js_boolean(&num_expr(-0.0)), Some(false));
  assert_eq!(to_js_boolean(&bool_expr(false)), Some(false));
  assert_eq!(to_js_boolean(&null_expr()), Some(false));
  assert_eq!(to_js_boolean(&ident_expr("undefined")), Some(false));
  // `NaN` reaches the coercion two ways — as the global it was written as, and
  // as the number a fold arrived at — and is falsy by both routes. The second
  // is the one an inequality against zero answers wrongly.
  assert_eq!(to_js_boolean(&ident_expr("NaN")), Some(false));
  assert_eq!(to_js_boolean(&num_expr(f64::NAN)), Some(false));
  assert_eq!(to_js_boolean(&void_expr(num_expr(0.0))), Some(false));
}

#[test]
fn a_primitive_that_is_not_on_that_list_is_truthy() {
  // `'0'` and `'false'` are the two that catch a reader out: a non-empty
  // string is truthy whatever it spells.
  assert_eq!(to_js_boolean(&str_expr("0")), Some(true));
  assert_eq!(to_js_boolean(&str_expr("false")), Some(true));
  assert_eq!(to_js_boolean(&num_expr(-1.0)), Some(true));
  assert_eq!(to_js_boolean(&num_expr(f64::INFINITY)), Some(true));
  assert_eq!(to_js_boolean(&bool_expr(true)), Some(true));
  assert_eq!(to_js_boolean(&ident_expr("Infinity")), Some(true));
}

#[test]
fn the_two_spellings_of_a_value_agree_on_its_boolean_and_its_nullishness() {
  // `void x` and `NaN` each reach a coercion two ways, and a caller that folds
  // `??` on one spelling and refuses `||` on the other would disagree about a
  // value the language does not.
  for expr in [void_expr(num_expr(0.0)), ident_expr("undefined")] {
    assert_eq!(to_js_boolean(&expr), Some(false));
    assert!(is_nullish(&expr));
  }

  for expr in [num_expr(f64::NAN), ident_expr("NaN")] {
    assert_eq!(to_js_boolean(&expr), Some(false));
    assert!(!is_nullish(&expr));
  }
}

#[test]
fn zero_is_the_only_falsy_big_integer() {
  assert_eq!(to_js_boolean(&big_int_expr(0)), Some(false));
  assert_eq!(to_js_boolean(&big_int_expr(1)), Some(true));
  assert_eq!(to_js_boolean(&big_int_expr(-1)), Some(true));
}

#[test]
fn every_object_is_truthy_however_empty_it_is() {
  assert_eq!(to_js_boolean(&empty_object_expr()), Some(true));
  assert_eq!(to_js_boolean(&array_expr(vec![])), Some(true));
  assert_eq!(to_js_boolean(&regex_expr("", "")), Some(true));
  assert_eq!(to_js_boolean(&arrow_expr()), Some(true));
}

#[test]
fn an_own_conversion_method_does_not_enter_into_the_boolean() {
  // `ToBoolean` is the one coercion that never reaches `ToPrimitive`, so an
  // object whose `toString` answers the empty string is still truthy — where
  // its string coercion is the falsy value that method returns.
  let overriding = object_expr(vec![key_value_prop(
    ident_key("toString"),
    returning_arrow(str_expr("")),
  )]);

  assert_eq!(to_js_string(&overriding).as_deref(), Some(""));
  assert_eq!(to_js_boolean(&overriding), Some(true));
}

#[test]
fn an_object_with_no_string_form_still_has_a_boolean() {
  // A spread makes the elements unnameable and so the join unknowable, but an
  // array is truthy without any of them being read.
  let spread = spread_array_expr(array_expr(vec![]));

  assert_eq!(to_js_string(&spread), None);
  assert_eq!(to_js_boolean(&spread), Some(true));
}

#[test]
fn a_value_of_no_readable_kind_has_no_boolean() {
  // Which side of the falsy list this falls on cannot be read off it, so the
  // caller deopts rather than picking one.
  assert_eq!(to_js_boolean(&ident_expr("someBinding")), None);
  assert_eq!(to_js_boolean(&this_expr()), None);
}

#[test]
fn the_nullish_values_are_null_undefined_and_void() {
  assert!(is_nullish(&null_expr()));
  assert!(is_nullish(&ident_expr("undefined")));
  // `void` yields `undefined` whatever it is applied to, so the operand is
  // never read.
  assert!(is_nullish(&void_expr(num_expr(0.0))));
  assert!(is_nullish(&void_expr(str_expr("red"))));
}

#[test]
fn a_falsy_value_that_is_not_nullish_says_so() {
  // The distinction `??` rests on, and the one `||` does not draw: each of
  // these is falsy and none of them is nullish.
  assert!(!is_nullish(&str_expr("")));
  assert!(!is_nullish(&num_expr(0.0)));
  assert!(!is_nullish(&bool_expr(false)));
  assert!(!is_nullish(&ident_expr("NaN")));
}

#[test]
fn a_value_this_crate_cannot_read_is_not_nullish() {
  // Both spellings of nullish are syntax the predicate always recognises, so
  // anything else is answered rather than refused.
  assert!(!is_nullish(&ident_expr("someBinding")));
  assert!(!is_nullish(&this_expr()));
  assert!(!is_nullish(&empty_object_expr()));
  assert!(!is_nullish(&arrow_expr()));
}

// ==================== the global set, read as a set ====================

/// The three names the language spells as an identifier rather than as a
/// literal, and the only three.
///
/// Asked as a set rather than through a coercion because that is how the
/// evaluator asks: its reference-resolution chain has to know whether a name
/// *could* be one of these before it can decide whether something in scope took
/// the name over, and its object coercion answers that all three carry no own
/// properties. Neither wants a string back.
#[test]
fn the_three_globals_are_spelled_as_identifiers() {
  assert!(is_global_spelled_as_an_identifier(&ident("undefined")));
  assert!(is_global_spelled_as_an_identifier(&ident("NaN")));
  assert!(is_global_spelled_as_an_identifier(&ident("Infinity")));
}

/// Nothing else is, including every neighbouring spelling. The comparison is on
/// the whole symbol, so a difference of case or of one character is an ordinary
/// binding name -- and a binding is exactly what the caller must not mistake for
/// the global.
#[test]
fn no_other_name_is_a_global_spelled_as_an_identifier() {
  for name in [
    "nan",
    "NAN",
    "NaNa",
    "aNaN",
    "infinity",
    "INFINITY",
    "Undefined",
    "undefined_",
    "_undefined",
    "undefined2",
    // Other globals, which are spelled as identifiers but are not values a
    // coercion can read.
    "Math",
    "String",
    "Number",
    "globalThis",
    // Spellings no source can produce, which the predicate still must not
    // accept: a name carrying whitespace, one carrying a zero-width space, and
    // the empty symbol.
    "NaN ",
    " NaN",
    "NaN\u{200b}",
    "",
  ] {
    assert!(
      !is_global_spelled_as_an_identifier(&ident(name)),
      "expected `{}` not to be a global spelled as an identifier",
      name
    );
  }
}

/// The predicate and the coercions read one set, which is the whole reason the
/// set is written down once. A name the predicate accepts has a string form; one
/// it rejects is a binding this crate cannot read, and answers no string at all.
///
/// Asserted as a pair, per name, so a fourth global added to one reader and not
/// the other fails here rather than at whichever call site notices first.
#[test]
fn the_predicate_and_the_string_coercion_read_the_same_set() {
  for (name, expected) in [
    ("undefined", "undefined"),
    ("NaN", "NaN"),
    ("Infinity", "Infinity"),
  ] {
    assert!(is_global_spelled_as_an_identifier(&ident(name)), "{}", name);
    assert_eq!(to_js_string(&ident_expr(name)).as_deref(), Some(expected));
  }

  for name in ["someBinding", "nan", "Math"] {
    assert!(
      !is_global_spelled_as_an_identifier(&ident(name)),
      "{}",
      name
    );
    assert_eq!(to_js_string(&ident_expr(name)), None, "{}", name);
  }
}

/// Every value here was read out of `node -e 'console.log(x | 0)'`, which is
/// `ToInt32` spelled the shortest way, rather than derived from the
/// specification text.
#[test]
fn to_int32_wraps_into_the_signed_32_bit_range() {
  // The reason this function exists: a 64-bit negation answers -4294967297 for
  // `~[4294967296]`, where JavaScript answers -1.
  assert_eq!(to_int32(4_294_967_296.0), 0);
  assert_eq!(to_int32(2_147_483_648.0), -2_147_483_648);
  assert_eq!(to_int32(-2_147_483_649.0), 2_147_483_647);
  assert_eq!(to_int32(3_000_000_000.0), -1_294_967_296);
  assert_eq!(to_int32(1e21), -559_939_584);

  // Truncation is toward zero, not flooring.
  assert_eq!(to_int32(1.9), 1);
  assert_eq!(to_int32(-1.9), -1);

  // The values with no integer to wrap all answer zero rather than refusing.
  assert_eq!(to_int32(f64::NAN), 0);
  assert_eq!(to_int32(f64::INFINITY), 0);
  assert_eq!(to_int32(f64::NEG_INFINITY), 0);
  assert_eq!(to_int32(0.0), 0);
  assert_eq!(to_int32(-0.0), 0);

  // Inside the range, it is the identity on integers.
  assert_eq!(to_int32(2_147_483_647.0), 2_147_483_647);
  assert_eq!(to_int32(-2_147_483_648.0), -2_147_483_648);
  assert_eq!(to_int32(-1.0), -1);
}

/// Every value here was read out of `node -e 'console.log(x >>> 0)'`, which is
/// `ToUint32` spelled the shortest way.
#[test]
fn to_uint32_wraps_into_the_unsigned_32_bit_range() {
  // The reason this function exists: `>>>` reads its left side as unsigned, so
  // a negative number becomes the large one the language answers with.
  assert_eq!(to_uint32(-1.0), 4_294_967_295);
  assert_eq!(to_uint32(-9.0), 4_294_967_287);
  assert_eq!(to_uint32(-2_147_483_648.0), 2_147_483_648);

  // Above the range it wraps, exactly as `to_int32` does.
  assert_eq!(to_uint32(4_294_967_296.0), 0);
  assert_eq!(to_uint32(4_294_967_297.0), 1);
  assert_eq!(to_uint32(3_000_000_000.0), 3_000_000_000);

  // The values with no integer to wrap all answer zero rather than refusing.
  assert_eq!(to_uint32(f64::NAN), 0);
  assert_eq!(to_uint32(f64::INFINITY), 0);
  assert_eq!(to_uint32(f64::NEG_INFINITY), 0);
  assert_eq!(to_uint32(-0.0), 0);

  // Truncation is toward zero here too.
  assert_eq!(to_uint32(1.9), 1);
  assert_eq!(to_uint32(-1.9), 4_294_967_295);
}

/// Read out of `node -e 'console.log(1 << x)'`: the language keeps the low five
/// bits of a shift count and nothing else.
#[test]
fn to_shift_count_keeps_the_low_five_bits() {
  assert_eq!(to_shift_count(0.0), 0);
  assert_eq!(to_shift_count(31.0), 31);

  // A count of the word width shifts by nothing, which is where a Rust shift
  // panics in a debug build instead.
  assert_eq!(to_shift_count(32.0), 0);
  assert_eq!(to_shift_count(33.0), 1);
  assert_eq!(to_shift_count(64.0), 0);

  // A negative count wraps through `ToUint32` first, so `-1` is a shift of 31.
  assert_eq!(to_shift_count(-1.0), 31);
  assert_eq!(to_shift_count(-32.0), 0);

  // The values with no integer answer a shift of nothing.
  assert_eq!(to_shift_count(f64::NAN), 0);
  assert_eq!(to_shift_count(f64::INFINITY), 0);
  assert_eq!(to_shift_count(2.9), 2);
}

// ── the equality readings ────────────────────────────────────────────

/// The five primitives this evaluator holds, read off the expressions that
/// spell them -- including the three globals the language spells as a name.
#[test]
fn to_js_primitive_reads_every_primitive_spelling() {
  assert!(matches!(
    to_js_primitive(&str_expr("a")),
    Some(Primitive::String(_))
  ));
  assert!(matches!(
    to_js_primitive(&num_expr(1.0)),
    Some(Primitive::Number(value)) if value == 1.0
  ));
  assert!(matches!(
    to_js_primitive(&bool_expr(true)),
    Some(Primitive::Boolean(true))
  ));
  assert!(matches!(
    to_js_primitive(&null_expr()),
    Some(Primitive::Null)
  ));
  assert!(matches!(
    to_js_primitive(&ident_expr("undefined")),
    Some(Primitive::Undefined)
  ));
  assert!(matches!(
    to_js_primitive(&ident_expr("NaN")),
    Some(Primitive::Number(value)) if value.is_nan()
  ));
  assert!(matches!(
    to_js_primitive(&ident_expr("Infinity")),
    Some(Primitive::Number(value)) if value.is_infinite()
  ));
  assert!(matches!(
    to_js_primitive(&void_expr(num_expr(1.0))),
    Some(Primitive::Undefined)
  ));
}

/// Everything else has no primitive to compare: the language compares two
/// objects by reference, and this evaluator holds a copy rather than a
/// reference.
#[test]
fn to_js_primitive_refuses_every_value_that_is_not_one() {
  for expr in [
    Expr::Object(ObjectLit {
      span: DUMMY_SP,
      props: vec![],
    }),
    Expr::Array(ArrayLit {
      span: DUMMY_SP,
      elems: vec![],
    }),
    arrow_expr(),
    ident_expr("someBinding"),
    Expr::Lit(Lit::BigInt(BigInt {
      span: DUMMY_SP,
      value: Box::new(1.into()),
      raw: None,
    })),
    Expr::Lit(Lit::Regex(Regex {
      span: DUMMY_SP,
      exp: "a".into(),
      flags: "".into(),
    })),
  ] {
    assert!(to_js_primitive(&expr).is_none(), "{:?}", expr);
  }
}

/// Read out of `node -e "console.log(a === b)"`. Two values of different types
/// are unequal with no coercion at all, and `NaN` equals nothing.
#[test]
fn strict_equals_compares_the_type_before_the_value() {
  let cases: [(Expr, Expr, bool); 12] = [
    (num_expr(1.0), num_expr(1.0), true),
    (num_expr(1.0), num_expr(2.0), false),
    (num_expr(0.0), num_expr(-0.0), true),
    (ident_expr("NaN"), ident_expr("NaN"), false),
    (str_expr("a"), str_expr("a"), true),
    (str_expr("a"), str_expr("b"), false),
    (bool_expr(true), bool_expr(true), true),
    (null_expr(), null_expr(), true),
    (ident_expr("undefined"), ident_expr("undefined"), true),
    // The four rows a coercion would have folded together.
    (num_expr(1.0), str_expr("1"), false),
    (null_expr(), ident_expr("undefined"), false),
    (bool_expr(true), num_expr(1.0), false),
  ];

  for (left, right, expected) in cases {
    let (left_value, right_value) = primitive_pair(&left, &right);

    assert_eq!(
      strict_equals(&left_value, &right_value),
      expected,
      "{:?} === {:?}",
      left,
      right
    );
  }
}

/// Read out of `node -e "console.log(a == b)"`. The three coercions the
/// algorithm applies, and the rows where it applies none.
#[test]
fn loose_equals_applies_the_three_coercions_and_no_others() {
  let cases: [(Expr, Expr, bool); 14] = [
    // A string meeting a number becomes its number.
    (num_expr(1.0), str_expr("1"), true),
    (str_expr("1"), num_expr(1.0), true),
    (num_expr(1.0), str_expr("1px"), false),
    (num_expr(0.0), str_expr(""), true),
    (num_expr(0.0), str_expr("  "), true),
    // A boolean becomes its number on whichever side it is.
    (bool_expr(true), num_expr(1.0), true),
    (num_expr(0.0), bool_expr(false), true),
    (bool_expr(true), str_expr("1"), true),
    // The two nullish values equal each other and nothing else.
    (null_expr(), ident_expr("undefined"), true),
    (null_expr(), num_expr(0.0), false),
    (ident_expr("undefined"), str_expr(""), false),
    (null_expr(), bool_expr(false), false),
    // Same type is the strict reading, `NaN` included.
    (str_expr("a"), str_expr("a"), true),
    (ident_expr("NaN"), ident_expr("NaN"), false),
  ];

  for (left, right, expected) in cases {
    let (left_value, right_value) = primitive_pair(&left, &right);

    assert_eq!(
      loose_equals(&left_value, &right_value),
      expected,
      "{:?} == {:?}",
      left,
      right
    );
  }
}

/// Two strings that hold different lone surrogates are different strings, which
/// is why the reading holds the atom rather than the text: `as_str` reads both
/// as one replacement character.
#[test]
fn two_lone_surrogates_are_not_the_same_string() {
  let high = lone_surrogate_expr(0xD83D);
  let low = lone_surrogate_expr(0xDE00);

  let (high_value, low_value) = primitive_pair(&high, &low);
  assert!(!strict_equals(&high_value, &low_value));
  assert!(!loose_equals(&high_value, &low_value));

  let same_high = lone_surrogate_expr(0xD83D);
  let (high_value, same) = primitive_pair(&high, &same_high);
  assert!(strict_equals(&high_value, &same));

  // A text with no `str` is not a numeric literal, so it meets a number as
  // `NaN` rather than refusing.
  let zero_expr = num_expr(0.0);
  let (high_value, zero) = primitive_pair(&high, &zero_expr);
  assert!(!loose_equals(&high_value, &zero));
}

/// Read out of `node -e "console.log(a < b)"`. Two strings compare by code
/// unit and every other pair compares as two numbers.
#[test]
fn js_less_than_orders_two_strings_by_code_unit_and_everything_else_by_number() {
  let cases: [(Expr, Expr, bool); 12] = [
    // The rows a numeric reading gets wrong, because both sides are strings.
    (str_expr("10"), str_expr("9"), true),
    (str_expr("2"), str_expr("10"), false),
    (str_expr("a"), str_expr("b"), true),
    (str_expr(""), str_expr("a"), true),
    (str_expr("a"), str_expr(""), false),
    // One string and one number is the numeric reading, so the text is read
    // through `StringToNumber` rather than by code unit.
    (str_expr("10"), num_expr(9.0), false),
    (num_expr(9.0), str_expr("10"), true),
    // The nullish pair: `null` is nought and `undefined` is `NaN`.
    (null_expr(), num_expr(1.0), true),
    (num_expr(1.0), null_expr(), false),
    // A boolean is its number, on whichever side it is.
    (bool_expr(false), num_expr(1.0), true),
    (num_expr(1.0), bool_expr(true), false),
    // The two zeroes are neither less than nor greater than each other.
    (num_expr(0.0), num_expr(-0.0), false),
  ];

  for (left, right, expected) in cases {
    let (left_value, right_value) = primitive_pair(&left, &right);

    assert_eq!(
      js_less_than(&left_value, &right_value),
      Some(Some(expected)),
      "{:?} < {:?}",
      left,
      right
    );
  }
}

/// A `NaN` on either side has no ordering at all, which is the `undefined` the
/// specification names -- and what makes all four relational operators false.
#[test]
fn js_less_than_has_no_answer_for_a_not_a_number_operand() {
  let not_a_number = ident_expr("NaN");
  let one = num_expr(1.0);

  let (left, right) = primitive_pair(&not_a_number, &one);
  assert_eq!(js_less_than(&left, &right), Some(None));

  let (left, right) = primitive_pair(&one, &not_a_number);
  assert_eq!(js_less_than(&left, &right), Some(None));

  // A text with no numeric literal in it is `NaN` too, so it has no ordering
  // against a number -- but it still orders against another string.
  let text = str_expr("10px");
  let (left, right) = primitive_pair(&text, &one);
  assert_eq!(js_less_than(&left, &right), Some(None));

  // `undefined` is the third spelling of `NaN` here, and the one that separates
  // it from `null`: `null < 1` is true and `undefined < 1` is false.
  let absent = ident_expr("undefined");
  let (left, right) = primitive_pair(&absent, &one);
  assert_eq!(js_less_than(&left, &right), Some(None));

  let (left, right) = primitive_pair(&one, &absent);
  assert_eq!(js_less_than(&left, &right), Some(None));
}

/// A string this crate cannot read has no ordering, and the refusal travels out
/// rather than being answered as `false`: `'\u{D83D}' < 'a'` is a question with
/// a real answer, and guessing it would put the wrong arm of a conditional in
/// the stylesheet.
///
/// It refuses only where the code-unit rule claims the pair. Against a number
/// the same text is read as `NaN`, which every text with no numeric literal in
/// it already is.
#[test]
fn js_less_than_refuses_a_string_it_cannot_read() {
  let surrogate = lone_surrogate_expr(0xD83D);
  let text = str_expr("a");
  let one = num_expr(1.0);

  let (left, right) = primitive_pair(&surrogate, &text);
  assert_eq!(js_less_than(&left, &right), None);

  let (left, right) = primitive_pair(&text, &surrogate);
  assert_eq!(js_less_than(&left, &right), None);

  let (left, right) = primitive_pair(&surrogate, &one);
  assert_eq!(js_less_than(&left, &right), Some(None));
}

/// Code unit rather than code point, which is the one row the two orderings
/// disagree on: an astral character is a surrogate pair whose first unit is
/// below `U+E000`, so it sorts *before* a character the code-point order puts
/// first.
#[test]
fn js_less_than_orders_an_astral_character_by_its_first_code_unit() {
  let astral = str_expr("\u{1F600}");
  let private_use = str_expr("\u{E000}");

  let (left, right) = primitive_pair(&astral, &private_use);
  assert_eq!(js_less_than(&left, &right), Some(Some(true)));
}

/// `ToPrimitive` with no hint asks an object for its own `valueOf` first, which
/// is the reduction `+` applies before it decides whether it is addition or
/// concatenation.
#[test]
fn to_js_default_primitive_prefers_an_own_value_of() {
  let both = object_expr(vec![
    key_value_prop(ident_key("valueOf"), returning_arrow(num_expr(2.0))),
    key_value_prop(ident_key("toString"), returning_arrow(str_expr("9"))),
  ]);

  assert_eq!(
    to_js_default_primitive(&both)
      .and_then(to_js_string)
      .as_deref(),
    Some("2")
  );

  // `toString` alone still answers, because the order is a preference rather
  // than a requirement.
  let to_string_only = object_expr(vec![key_value_prop(
    ident_key("toString"),
    returning_arrow(str_expr("5")),
  )]);

  assert_eq!(
    to_js_default_primitive(&to_string_only)
      .and_then(to_js_string)
      .as_deref(),
    Some("5")
  );
}

/// A value that is already a primitive answers itself, whatever it is. So does
/// every value that is not an object at all, which is what keeps the reduction
/// a no-op for every side but the one it was written for.
#[test]
fn to_js_default_primitive_answers_a_primitive_with_itself() {
  for expr in [
    str_expr("a"),
    num_expr(1.0),
    bool_expr(true),
    null_expr(),
    ident_expr("undefined"),
  ] {
    match to_js_default_primitive(&expr) {
      Some(answered) => assert!(std::ptr::eq(answered, &expr), "{:?}", expr),
      None => panic!("{:?} is already a primitive", expr),
    }
  }
}

/// An object that keeps the `Object.prototype` pair reduces to nothing: its
/// primitive is the default text, which the caller's string path already
/// writes.
#[test]
fn to_js_default_primitive_leaves_the_default_pair_to_the_string_path() {
  assert!(to_js_default_primitive(&object_expr(vec![])).is_none());

  // A method answering an object has answered no primitive, and the other
  // method is the default one -- so there is no value to fold.
  let answers_an_object = object_expr(vec![key_value_prop(
    ident_key("valueOf"),
    returning_arrow(object_expr(vec![])),
  )]);

  assert!(to_js_default_primitive(&answers_an_object).is_none());
}

/// A method answering another object keeps reducing, because the value a
/// conversion hands back is itself read through `ToPrimitive`.
#[test]
fn to_js_default_primitive_reduces_what_a_conversion_answers() {
  let nested = object_expr(vec![key_value_prop(
    ident_key("valueOf"),
    returning_arrow(str_expr("inner")),
  )]);

  assert_eq!(
    to_js_default_primitive(&nested)
      .and_then(to_js_string)
      .as_deref(),
    Some("inner")
  );
}

/// Both sides of a comparison, for a case that spells them as expressions.
#[track_caller]
fn primitive_pair<'a>(left: &'a Expr, right: &'a Expr) -> (Primitive<'a>, Primitive<'a>) {
  match (to_js_primitive(left), to_js_primitive(right)) {
    (Some(left), Some(right)) => (left, right),
    _ => panic!("both sides of a case have to be primitives"),
  }
}

/// A string literal holding one lone surrogate, which no Rust `str` can spell.
fn lone_surrogate_expr(code_unit: u16) -> Expr {
  let mut buffer = Wtf8Buf::new();

  match CodePoint::from_u32(u32::from(code_unit)) {
    Some(point) => buffer.push(point),
    None => panic!("{} is not a code point", code_unit),
  }

  Expr::Lit(Lit::Str(Str {
    span: DUMMY_SP,
    value: Wtf8Atom::from(buffer),
    raw: None,
  }))
}

// ── global_identifier_to_value ───────────────────────────────────────

/// The two numeric globals answer with the numbers they *are*, and `undefined`
/// answers with itself.
///
/// The split is not cosmetic. A consumer that inspects the expression's shape
/// rather than coercing it — style-value validation is the one that does, since
/// it admits a number and refuses an identifier — sees `NaN` as a number here,
/// so `height: [NaN, '2px']` is accepted exactly as `height: [0/0, '2px']` is.
/// `undefined` has no numeric reading to answer with, so it stands.
#[test]
fn global_identifier_to_value_answers_numbers_for_the_two_numeric_globals() {
  match global_identifier_to_value(&ident("NaN")) {
    Some(Expr::Lit(Lit::Num(number))) => {
      assert!(number.value.is_nan(), "NaN is the value, not the name");
      assert_eq!(
        number.raw.as_deref(),
        Some("NaN"),
        "and it keeps the text it was authored with"
      );
    },
    other => panic!("expected NaN to answer a number, got {:?}", other),
  }

  match global_identifier_to_value(&ident("Infinity")) {
    Some(Expr::Lit(Lit::Num(number))) => {
      assert_eq!(number.value, f64::INFINITY);
      assert_eq!(number.raw.as_deref(), Some("Infinity"));
    },
    other => panic!("expected Infinity to answer a number, got {:?}", other),
  }
}

/// `undefined` has no literal spelling, so the identifier is the answer. The
/// span comes back with it, which is what lets a caller report against the
/// reference it read.
#[test]
fn global_identifier_to_value_answers_undefined_with_itself() {
  match global_identifier_to_value(&ident("undefined")) {
    Some(Expr::Ident(answered)) => assert_eq!(answered.sym.as_ref(), "undefined"),
    other => panic!("expected undefined to answer itself, got {:?}", other),
  }
}

/// `None` for every other name, which is what lets a caller use this as the set
/// as well as the coercion — and it agrees with the predicate that publishes
/// the set on its own.
#[test]
fn global_identifier_to_value_declines_every_other_name() {
  for name in ["Number", "nan", "NAN", "infinity", "undefined_", "x", ""] {
    assert!(
      global_identifier_to_value(&ident(name)).is_none(),
      "`{}` is not one of the three globals",
      name
    );
    assert!(
      !is_global_spelled_as_an_identifier(&ident(name)),
      "`{}` must agree with the predicate over the same set",
      name
    );
  }

  for name in ["undefined", "NaN", "Infinity"] {
    assert!(
      global_identifier_to_value(&ident(name)).is_some(),
      "`{}` is one of the three",
      name
    );
    assert!(is_global_spelled_as_an_identifier(&ident(name)));
  }
}

/// The `raw` text is the point of `number_spelled_as`, not the value: asked to
/// print a `Number` node holding `NaN` with no raw text, the emitter writes
/// `0 / 0`, and `Infinity` becomes a numeral no author wrote. Both evaluate
/// correctly either way, so this pins the text a reader diffs.
#[test]
fn the_numeric_globals_carry_the_text_they_were_authored_with() {
  let raw_of = |name: &str| match global_identifier_to_value(&ident(name)) {
    Some(Expr::Lit(Lit::Num(number))) => number.raw.map(|raw| raw.to_string()),
    other => panic!("expected a number for `{}`, got {:?}", name, other),
  };

  assert_eq!(raw_of("NaN").as_deref(), Some("NaN"));
  assert_eq!(raw_of("Infinity").as_deref(), Some("Infinity"));
}

// ──────────────────────────────────────────────
// The streamed join
// ──────────────────────────────────────────────

/// A sink that takes a fixed number of code units and refuses the piece that
/// would pass it -- the shape of the character ceiling, without a compile.
///
/// It records what it took as well, so a case can assert that a refusal arrives
/// *before* the rest of the pieces are written rather than after.
struct Bounded {
  text: String,
  ceiling: usize,
}

impl Bounded {
  fn new(ceiling: usize) -> Self {
    Self {
      text: String::new(),
      ceiling,
    }
  }
}

impl StringSink for Bounded {
  type Refusal = usize;

  fn write(&mut self, piece: &str) -> Result<(), usize> {
    // UTF-16 code units, which is what the real ceiling spends -- so the stand-in
    // cannot pass a case the compiler's own sink would refuse.
    let grown = utf16_length(&self.text) + utf16_length(piece);

    if grown > self.ceiling {
      return Err(grown);
    }

    self.text.push_str(piece);

    Ok(())
  }
}

/// Streaming answers exactly what collecting did, over every shape an array's
/// join has a rule for: the separator between elements, the two values that join
/// as nothing, a hole, and nesting.
#[test]
fn a_streamed_join_writes_what_the_collected_one_answered() {
  let cases: &[(Expr, &str)] = &[
    (array_expr(vec![]), ""),
    (array_expr(vec![Some(str_expr("a"))]), "a"),
    (
      array_expr(vec![Some(str_expr("a")), Some(str_expr("b"))]),
      "a,b",
    ),
    (
      array_expr(vec![Some(null_expr()), Some(ident_expr("undefined"))]),
      ",",
    ),
    (array_expr(vec![None, Some(str_expr("a")), None]), ",a,"),
    (
      array_expr(vec![
        Some(array_expr(vec![Some(str_expr("a")), Some(str_expr("b"))])),
        Some(str_expr("c")),
      ]),
      "a,b,c",
    ),
    (
      array_expr(vec![Some(num_expr(1.0)), Some(bool_expr(true))]),
      "1,true",
    ),
  ];

  for (expr, expected) in cases {
    let mut streamed = String::new();

    assert!(
      write_js_string_of(expr, FunctionForm::Refuse, &mut streamed).is_ok(),
      "expected `{:?}` to have a string",
      expr
    );
    assert_eq!(streamed, *expected);
    assert_eq!(to_js_string(expr).as_deref(), Some(*expected));
  }
}

/// A refusal ends the join where it happens, so the pieces after it are never
/// written. That is the whole point of streaming: the elements a bounded caller
/// refuses are elements it never rendered.
#[test]
fn a_sink_refusal_ends_the_join_where_it_happens() {
  let array = array_expr(vec![
    Some(str_expr("aaa")),
    Some(str_expr("bbb")),
    Some(str_expr("ccc")),
  ]);

  let mut sink = Bounded::new(5);

  match write_js_string_of(&array, FunctionForm::Refuse, &mut sink) {
    // Four code units are held -- `aaa` and the separator -- and `bbb` would
    // make seven, which is the number the refusal carries.
    Err(StringRefusal::Sink(grown)) => assert_eq!(grown, 7),
    other => panic!("expected the sink to refuse, got {:?}", other),
  }

  assert_eq!(sink.text, "aaa,");
}

/// The separator counts against the sink as much as an element does, since it is
/// part of the string being built. Two single-character elements need three.
#[test]
fn the_separator_is_measured_with_the_elements() {
  let array = array_expr(vec![Some(str_expr("a")), Some(str_expr("b"))]);

  let mut exact = Bounded::new(3);
  assert!(write_js_string_of(&array, FunctionForm::Refuse, &mut exact).is_ok());
  assert_eq!(exact.text, "a,b");

  // Two admits the first element and the separator after it, and refuses the
  // second element -- so the separator is charged where it is written rather
  // than held back until an element follows it.
  let mut one_short = Bounded::new(2);
  assert!(matches!(
    write_js_string_of(&array, FunctionForm::Refuse, &mut one_short),
    Err(StringRefusal::Sink(_))
  ));
  assert_eq!(one_short.text, "a,");
}

/// An element with no string form is the join's other ending, and it is reported
/// as its own kind rather than as the sink's -- a caller that reads them alike
/// would tell an author about a ceiling where a function was written.
#[test]
fn an_element_with_no_string_form_is_not_a_sink_refusal() {
  let array = array_expr(vec![Some(str_expr("a")), Some(arrow_expr())]);

  let mut sink = Bounded::new(1000);

  assert!(matches!(
    write_js_string_of(&array, FunctionForm::Refuse, &mut sink),
    Err(StringRefusal::NoStringForm)
  ));

  // Under the number form the same function stands in for its source text, so
  // the join has an answer and the sink takes it.
  let mut counted = Bounded::new(1000);
  assert!(write_js_string_of(&array, FunctionForm::NotANumber, &mut counted).is_ok());
  assert_eq!(counted.text, "a,function");
}

/// A spread element is not a written element, so an array holding one has no
/// join at all -- and the pieces before it are what the sink was given, since a
/// refusal stops rather than rewinds.
#[test]
fn a_spread_element_has_no_join() {
  let array = Expr::Array(ArrayLit {
    span: DUMMY_SP,
    elems: vec![
      Some(ExprOrSpread {
        spread: None,
        expr: Box::new(str_expr("a")),
      }),
      Some(ExprOrSpread {
        spread: Some(DUMMY_SP),
        expr: Box::new(str_expr("b")),
      }),
    ],
  });

  let mut sink = Bounded::new(1000);

  assert!(matches!(
    write_js_string_of(&array, FunctionForm::Refuse, &mut sink),
    Err(StringRefusal::NoStringForm)
  ));
  assert_eq!(sink.text, "a,");
}

/// Nesting deep enough to be worth naming still answers one flat join, and the
/// sink sees no separator for a level that holds one element.
#[test]
fn nesting_flattens_into_one_join() {
  let mut nested = str_expr("a");

  for _ in 0..64 {
    nested = array_expr(vec![Some(nested)]);
  }

  let mut sink = Bounded::new(1);

  assert!(write_js_string_of(&nested, FunctionForm::Refuse, &mut sink).is_ok());
  assert_eq!(sink.text, "a");
}

/// A sink that takes nothing at all refuses the first piece, and a value with no
/// pieces to write is not a refusal -- an empty array writes nothing, so a
/// ceiling of zero admits it.
#[test]
fn a_sink_that_takes_nothing_still_admits_an_empty_join() {
  let mut sink = Bounded::new(0);
  assert!(write_js_string_of(&array_expr(vec![]), FunctionForm::Refuse, &mut sink).is_ok());
  assert_eq!(sink.text, "");

  let mut refuses = Bounded::new(0);
  assert!(matches!(
    write_js_string_of(&str_expr("a"), FunctionForm::Refuse, &mut refuses),
    Err(StringRefusal::Sink(_))
  ));
}

// ──────────────────────────────────────────────
// The streamed number
// ──────────────────────────────────────────────

/// `ToNumber` writes only where the value has no number of its own, so a caller
/// measuring the text is charged for nothing it did not have to read.
#[test]
fn a_value_with_its_own_number_writes_nothing() {
  for (expr, number) in [
    (num_expr(7.0), 7.0),
    (bool_expr(true), 1.0),
    (bool_expr(false), 0.0),
    (null_expr(), 0.0),
  ] {
    let mut sink = Bounded::new(0);

    assert_eq!(
      write_js_number_of(&expr, &mut sink),
      Ok(NumberOf::Value(number))
    );
    assert_eq!(sink.text, "");
  }
}

/// Everything else reaches its number through the text it renders, which is what
/// the sink is handed.
#[test]
fn a_value_without_one_reaches_its_number_through_the_sink() {
  let array = array_expr(vec![Some(str_expr("1")), Some(str_expr("2"))]);
  let mut sink = Bounded::new(8);

  assert_eq!(write_js_number_of(&array, &mut sink), Ok(NumberOf::Text));
  assert_eq!(sink.text, "1,2");
  assert!(string_to_js_number(&sink.text).is_nan());
}

/// The two answers together are the collecting coercion, so the wrapper and the
/// stream cannot come to disagree.
#[test]
fn the_streamed_number_agrees_with_the_collected_one() {
  let cases = [
    num_expr(7.0),
    bool_expr(true),
    null_expr(),
    str_expr("0x10"),
    str_expr("nope"),
    array_expr(vec![]),
    array_expr(vec![Some(num_expr(5.0))]),
    array_expr(vec![Some(num_expr(1.0)), Some(num_expr(2.0))]),
    object_expr(vec![]),
  ];

  for expr in cases {
    let mut text = String::new();
    let streamed = match write_js_number_of(&expr, &mut text) {
      Ok(NumberOf::Value(number)) => Some(number),
      Ok(NumberOf::Text) => Some(string_to_js_number(&text)),
      Err(_) => None,
    };

    let collected = to_js_number(&expr);

    assert_eq!(
      streamed.map(f64::to_bits),
      collected.map(f64::to_bits),
      "the two readings of `ToNumber` must agree"
    );
  }
}

/// The character test is sound: every character a numeric literal can hold
/// answers `true`, so a `false` proves the whole text is `NaN`.
#[test]
fn every_character_a_numeric_literal_holds_is_admitted() {
  let literals = [
    "0",
    "9",
    "0x1234567890abcdefABCDEF",
    "0X1F",
    "0o17",
    "0O7",
    "0b01",
    "0B1",
    "1.5",
    "+1",
    "-1",
    "1e10",
    "1E-10",
    "Infinity",
    "-Infinity",
    " \t\n\r\u{000B}\u{000C}\u{00A0}\u{FEFF}1 ",
  ];

  for literal in literals {
    for character in literal.chars() {
      assert!(
        can_appear_in_a_number(character),
        "`{}` appears in the numeric literal `{}` and must be admitted",
        character,
        literal
      );
    }
  }
}

/// And the characters that settle the answer, of which the separator between two
/// array elements is the one that matters.
#[test]
fn a_character_no_numeric_literal_holds_is_refused() {
  for character in [
    ',',
    'z',
    'q',
    '%',
    '(',
    '\u{0000}',
    '\u{1F600}',
    '_',
    '/',
    '*',
  ] {
    assert!(
      !can_appear_in_a_number(character),
      "`{}` appears in no numeric literal and must not be admitted",
      character
    );
  }
}

/// The separator is a piece the sink can refuse on its own, which is the case a
/// ceiling that admits every element still reaches: two one-character elements
/// under a ceiling of one hold the first and stop at the comma, before the
/// second element is ever rendered.
#[test]
fn a_refused_separator_ends_the_join_before_the_next_element() {
  let array = array_expr(vec![Some(str_expr("a")), Some(str_expr("b"))]);

  let mut sink = Bounded::new(1);

  match write_js_string_of(&array, FunctionForm::Refuse, &mut sink) {
    // One code unit is held and the separator would make two, which is the
    // number the refusal carries -- an element's own length is not in it.
    Err(StringRefusal::Sink(grown)) => assert_eq!(grown, 2),
    other => panic!("expected the separator to be refused, got {:?}", other),
  }

  assert_eq!(sink.text, "a");

  // The same array one code unit higher gets past the separator and refuses on
  // the element after it, so the ceiling above is the separator's alone.
  let mut wider = Bounded::new(2);

  assert!(matches!(
    write_js_string_of(&array, FunctionForm::Refuse, &mut wider),
    Err(StringRefusal::Sink(3))
  ));
  assert_eq!(wider.text, "a,");
}

/// A string literal holding a lone surrogate has no `str` for Rust to write, so
/// it is the value's own ending rather than the sink's -- the same deopt every
/// unreadable value gets, which leaves the caller free to name the property the
/// value sits on.
#[test]
fn a_string_with_no_utf8_form_has_no_string_form() {
  // A lone surrogate: valid WTF-8 storage, invalid UTF-8 decoding.
  let mut lone_surrogate = Wtf8Buf::new();

  match CodePoint::from_u32(0xd800) {
    Some(code_point) => lone_surrogate.push(code_point),
    None => panic!("U+D800 is within the code-point range."),
  }

  let unreadable = Expr::Lit(Lit::Str(Str {
    span: DUMMY_SP,
    value: Wtf8Atom::new(lone_surrogate),
    raw: None,
  }));

  let mut sink = Bounded::new(1000);

  assert!(matches!(
    write_js_string_of(&unreadable, FunctionForm::Refuse, &mut sink),
    Err(StringRefusal::NoStringForm)
  ));
  // A refusal writes nothing, so a bounded caller is charged for no part of it.
  assert_eq!(sink.text, "");

  // The collecting coercion answers the same nothing, and so does the value in
  // every position a string reaches: an array element and an object's key.
  assert_eq!(to_js_string(&unreadable), None);
  assert_eq!(
    to_js_string(&array_expr(vec![Some(unreadable.clone())])),
    None
  );

  let mut nested = String::new();
  assert!(matches!(
    write_js_string_of(
      &array_expr(vec![Some(str_expr("a")), Some(unreadable)]),
      FunctionForm::Refuse,
      &mut nested
    ),
    Err(StringRefusal::NoStringForm)
  ));
  // The join stops rather than rewinds, so what was written before the
  // unreadable element stays written.
  assert_eq!(nested, "a,");
}

/// An object with no primitive of its own reaches its number through the
/// `Object.prototype` text, and that text is a piece the sink may refuse -- so
/// the number form has the sink's ending too, not only the string form's.
#[test]
fn an_object_default_text_can_be_refused_by_the_sink() {
  let object = object_expr(vec![key_value_prop(
    PropName::Ident(IdentName::new("a".into(), DUMMY_SP)),
    num_expr(1.0),
  )]);

  let mut sink = Bounded::new(OBJECT_TO_STRING.len() - 1);

  assert!(matches!(
    write_js_number_of(&object, &mut sink),
    Err(StringRefusal::Sink(_))
  ));
  assert_eq!(sink.text, "");

  // One code unit wider the same object writes its text and answers through it,
  // which is `NaN` -- so the refusal above is the ceiling's and not the shape's.
  let mut exact = Bounded::new(OBJECT_TO_STRING.len());

  assert_eq!(write_js_number_of(&object, &mut exact), Ok(NumberOf::Text));
  assert_eq!(exact.text, OBJECT_TO_STRING);
  assert!(string_to_js_number(&exact.text).is_nan());
}

/// A regular expression reaches the sink as the four pieces it spells rather
/// than as one joined text, so a caller measuring against a ceiling refuses at
/// the piece that passes it -- and what it has already taken is what stands.
#[test]
fn a_regular_expression_is_written_piece_by_piece() {
  // One ceiling per piece, so each of the four is the one that refuses: the
  // opening delimiter, the source, the closing delimiter and the flags.
  for (ceiling, kept) in [(0, ""), (1, "/"), (4, "/a+b"), (5, "/a+b/")] {
    let mut narrow = Bounded::new(ceiling);

    assert!(
      matches!(
        write_js_string_of(&regex_expr("a+b", "gi"), FunctionForm::Refuse, &mut narrow),
        Err(StringRefusal::Sink(_))
      ),
      "a ceiling of {} took the whole text",
      ceiling
    );
    assert_eq!(narrow.text, kept);
  }

  // The whole text still reaches a sink wide enough for it.
  let mut wide = Bounded::new(7);

  assert_eq!(
    write_js_string_of(&regex_expr("a+b", "gi"), FunctionForm::Refuse, &mut wide),
    Ok(())
  );
  assert_eq!(wide.text, "/a+b/gi");
}

/// Every ending the number form has, reached through a sink that can refuse --
/// the recursion into an own method's answer, the value with no string form at
/// all, and the sink's own refusal on the text a non-object renders.
///
/// The bounded sink is walked through all of them rather than only the ones a
/// ceiling is interesting for, because a sink that answers on some endings and
/// not others is a sink whose behaviour depends on which arm the value took.
#[test]
fn the_number_form_reaches_every_ending_through_a_bounded_sink() {
  // An own `valueOf` answers a primitive, and the coercion carries on into it:
  // the number is that answer's, and no text was written to reach it.
  let returning = object_expr(vec![key_value_prop(
    ident_key("valueOf"),
    returning_arrow(num_expr(2.0)),
  )]);

  let mut answered = Bounded::new(0);

  assert_eq!(
    write_js_number_of(&returning, &mut answered),
    Ok(NumberOf::Value(2.0))
  );
  assert_eq!(answered.text, "");

  // A method in a form this crate cannot apply is the value's own ending, not
  // the sink's -- a caller that read them alike would name a ceiling where a
  // `TypeError` was written.
  let unapplicable = object_expr(vec![method_prop("valueOf")]);

  let mut refused = Bounded::new(1000);

  assert_eq!(
    write_js_number_of(&unapplicable, &mut refused),
    Err(StringRefusal::NoStringForm)
  );
  assert_eq!(refused.text, "");

  // A BigInt has no number. The refusal is the value's own, and the sink gets
  // no text.
  let mut big_integer = Bounded::new(1000);

  assert_eq!(
    write_js_number_of(&big_int_expr(2), &mut big_integer),
    Err(StringRefusal::NoStringForm)
  );
  assert_eq!(big_integer.text, "");

  // And a value that is neither a number nor an object reaches its number
  // through the text it renders, which the sink may refuse part-way.
  let mut narrow = Bounded::new(2);

  assert!(matches!(
    write_js_number_of(&str_expr("abcdef"), &mut narrow),
    Err(StringRefusal::Sink(_))
  ));
  assert_eq!(narrow.text, "");

  // One code unit wider than the text the same value renders, the sink takes it
  // and the number is read back through what was written.
  let mut wide = Bounded::new(6);

  assert_eq!(
    write_js_number_of(&str_expr("abcdef"), &mut wide),
    Ok(NumberOf::Text)
  );
  assert_eq!(wide.text, "abcdef");
  assert!(string_to_js_number(&wide.text).is_nan());
}

/// `ToNumber` of a BigInt throws a `TypeError`, so a BigInt has no number. This
/// is true for a BigInt that an own conversion method answers too: the method
/// is not evaluated, and its body is read as it stands. The string form is not
/// changed, because `ToString` of a BigInt is its digits.
#[test]
fn a_big_integer_has_no_number_where_it_is_read_as_one() {
  let returning = |method: &str| {
    object_expr(vec![key_value_prop(
      ident_key(method),
      returning_arrow(big_int_expr(2)),
    )])
  };

  for expr in [big_int_expr(2), returning("valueOf"), returning("toString")] {
    assert_eq!(to_js_number(&expr), None, "{expr:?}");
  }

  assert_eq!(to_js_string(&big_int_expr(2)).as_deref(), Some("2"));
}

/// `OrdinaryToPrimitive` skips an own method that cannot be called, and one
/// that answers an object. It then asks the other method. Only an object whose
/// two methods give no primitive has no conversion, because there the
/// language throws a `TypeError`. Each value is the answer of Node.
#[test]
fn a_conversion_method_that_gives_no_primitive_is_skipped() {
  let method = |name: &str, value: Expr| key_value_prop(ident_key(name), value);

  // The number asks for `valueOf` first, skips it, and reaches `toString`.
  for skipped in [
    num_expr(1.0),
    null_expr(),
    str_expr("x"),
    object_expr(vec![]),
    array_expr(vec![]),
    returning_arrow(object_expr(vec![])),
    // The parentheses that `() => ({})` needs in source.
    returning_arrow(Expr::Paren(ParenExpr {
      span: DUMMY_SP,
      expr: Box::new(object_expr(vec![])),
    })),
    returning_arrow(array_expr(vec![Some(num_expr(5.0))])),
  ] {
    let with_default = object_expr(vec![method("valueOf", skipped.clone())]);
    assert!(
      to_js_number(&with_default).is_some_and(f64::is_nan),
      "{with_default:?}"
    );

    let with_own = object_expr(vec![
      method("valueOf", skipped),
      method("toString", returning_arrow(str_expr("3"))),
    ]);
    assert_eq!(to_js_number(&with_own), Some(3.0), "{with_own:?}");
  }

  // The string asks for `toString` first, skips it, and reaches `valueOf`.
  let string_skips = object_expr(vec![
    method("toString", num_expr(2.0)),
    method("valueOf", returning_arrow(str_expr("v"))),
  ]);
  assert_eq!(to_js_string(&string_skips).as_deref(), Some("v"));

  // Both methods give no primitive, so the language throws.
  for no_primitive in [
    object_expr(vec![method("toString", num_expr(1.0))]),
    object_expr(vec![
      method("valueOf", num_expr(1.0)),
      method("toString", num_expr(1.0)),
    ]),
    object_expr(vec![method(
      "toString",
      returning_arrow(object_expr(vec![])),
    )]),
  ] {
    assert_eq!(to_js_number(&no_primitive), None, "{no_primitive:?}");
    assert_eq!(to_js_string(&no_primitive), None, "{no_primitive:?}");
  }
}

/// A name can hold a function or a value, so a method that a name holds cannot
/// be read off the expression. A coercion that asks for that method refuses,
/// and does not guess. A string asks for `toString` first, so an own `valueOf`
/// does not enter into it.
#[test]
fn a_conversion_method_held_by_a_name_has_no_coercion() {
  let held = |name: &str| object_expr(vec![key_value_prop(ident_key(name), ident_expr("convert"))]);

  assert_eq!(to_js_number(&held("valueOf")), None);
  assert_eq!(
    to_js_string(&held("valueOf")).as_deref(),
    Some(OBJECT_TO_STRING)
  );
  assert_eq!(to_js_number(&held("toString")), None);
  assert_eq!(to_js_string(&held("toString")), None);
}

/// No method gives a primitive, so the language throws a `TypeError`. The
/// reduction refuses, which is not the default text that it leaves to the
/// string path.
#[test]
fn to_js_default_primitive_refuses_an_object_with_no_primitive() {
  let no_primitive = object_expr(vec![key_value_prop(ident_key("toString"), num_expr(1.0))]);

  assert_eq!(to_js_default_primitive(&no_primitive), None);
  assert_eq!(to_js_string(&no_primitive), None);
}

/// A regular expression is an object, so a method that answers one gives no
/// primitive. The reference implementation refuses every regular expression
/// literal, so a coercion that asks a method that holds one refuses too. A
/// string asks for `toString` first, so an own `valueOf` does not enter into
/// it.
#[test]
fn a_regular_expression_in_a_conversion_method_has_no_coercion() {
  for value in [returning_arrow(regex_expr("a", "")), regex_expr("a", "")] {
    let to_string = object_expr(vec![key_value_prop(ident_key("toString"), value.clone())]);
    let value_of = object_expr(vec![key_value_prop(ident_key("valueOf"), value)]);

    assert_eq!(to_js_number(&to_string), None);
    assert_eq!(to_js_string(&to_string), None);
    assert_eq!(to_js_number(&value_of), None);
    assert_eq!(to_js_string(&value_of).as_deref(), Some(OBJECT_TO_STRING));
  }
}

/// The last of two own keys with one name is the property, as in the language.
/// A method in parentheses is the same method.
#[test]
fn a_conversion_method_is_the_last_own_key_read_through_parentheses() {
  let paren = |expr: Expr| {
    Expr::Paren(ParenExpr {
      span: DUMMY_SP,
      expr: Box::new(expr),
    })
  };
  let value_of = |value: Expr| key_value_prop(ident_key("valueOf"), value);

  let last_answers = object_expr(vec![
    value_of(num_expr(1.0)),
    value_of(returning_arrow(num_expr(2.0))),
  ]);
  assert_eq!(to_js_number(&last_answers), Some(2.0));

  let last_skipped = object_expr(vec![
    value_of(returning_arrow(num_expr(2.0))),
    value_of(num_expr(1.0)),
  ]);
  assert!(to_js_number(&last_skipped).is_some_and(f64::is_nan));

  let wrapped = object_expr(vec![value_of(paren(returning_arrow(num_expr(2.0))))]);
  assert_eq!(to_js_number(&wrapped), Some(2.0));

  let wrapped_value = object_expr(vec![value_of(paren(num_expr(1.0)))]);
  assert!(to_js_number(&wrapped_value).is_some_and(f64::is_nan));
}
