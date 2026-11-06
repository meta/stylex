// Source: crates/stylex-js/src/coercions.rs

use super::*;
use swc_core::{
  atoms::{
    Wtf8Atom,
    wtf8::{CodePoint, Wtf8Buf},
  },
  common::DUMMY_SP,
  ecma::ast::{
    ArrayLit, ArrowExpr, ArrowFunctionBody, AssignProp, BigInt, BindingIdent, Bool,
    ComputedPropName, ExprOrSpread, Function, FunctionBody, GetterProp, Ident, IdentName,
    KeyValueProp, MethodProp, Null, Number, ObjectLit, Pat, Prop, PropName, PropOrSpread, Regex,
    SetterProp, SpreadElement, Str, ThisExpr, UnaryExpr,
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
fn a_regular_expression_renders_its_own_source() {
  assert_eq!(to_js_string(&regex_expr("x", "")).as_deref(), Some("/x/"));
  assert_eq!(
    to_js_string(&regex_expr("a+b", "gi")).as_deref(),
    Some("/a+b/gi")
  );
  assert!(
    to_js_number(&regex_expr("x", ""))
      .expect("a regular expression has a number")
      .is_nan()
  );
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

/// A zero-parameter arrow returning `body`.
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
  let overriding = object_expr(vec![key_value_prop(
    ident_key("toString"),
    returning_arrow(str_expr("red")),
  )]);

  assert_eq!(to_js_string(&overriding).as_deref(), Some("red"));

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
fn an_object_whose_keys_cannot_be_named_has_no_coercion() {
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
fn the_radix_prefixes_are_read_as_javascript_reads_them() {
  assert_eq!(string_to_js_number("0x1f"), 31.0);
  assert_eq!(string_to_js_number("0X1F"), 31.0);
  assert_eq!(string_to_js_number("0b101"), 5.0);
  assert_eq!(string_to_js_number("0B101"), 5.0);
  assert_eq!(string_to_js_number("0o17"), 15.0);
  assert_eq!(string_to_js_number("0O17"), 15.0);
  assert_eq!(string_to_js_number("  0x1f  "), 31.0);
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
  assert!(string_to_js_number("-0x1f").is_nan());
  assert!(string_to_js_number("+0x1f").is_nan());
}

#[test]
fn every_value_but_a_function_coerces_to_a_plain_object() {
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
  assert_eq!(to_object(&ident_expr("NaN")), Some(ObjectCoercion::Object));
  assert_eq!(
    to_object(&ident_expr("Infinity")),
    Some(ObjectCoercion::Object)
  );
}

/// An unresolved `this` expression.
fn this_expr() -> Expr {
  Expr::This(ThisExpr { span: DUMMY_SP })
}

fn binding_pat(name: &str) -> Pat {
  Pat::Ident(BindingIdent {
    id: Ident::new(name.into(), DUMMY_SP, Default::default()),
    type_ann: None,
  })
}

/// A one-parameter arrow returning `body`.
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

/// A block-bodied arrow.
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
  assert_eq!(to_js_string(&this_expr()), None);
  assert_eq!(to_js_number(&this_expr()), None);
  assert_eq!(to_object(&this_expr()), None);
}

#[test]
fn a_conversion_method_written_as_a_method_or_an_accessor_has_no_coercion() {
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
  assert_eq!(
    to_js_string(&object_expr(vec![shorthand_prop("toString")])),
    None
  );
  assert_eq!(
    to_js_string(&object_expr(vec![assign_prop("toString", str_expr("red"))])),
    None
  );
  assert_eq!(
    to_js_string(&object_expr(vec![shorthand_prop("a")])).as_deref(),
    Some(OBJECT_TO_STRING)
  );
}

#[test]
fn a_key_that_is_not_a_name_is_not_a_conversion_method() {
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

fn void_expr(operand: Expr) -> Expr {
  Expr::Unary(UnaryExpr {
    span: DUMMY_SP,
    op: UnaryOp::Void,
    arg: Box::new(operand),
  })
}

#[test]
fn a_primitive_that_is_not_on_that_list_is_truthy() {
  assert_eq!(to_js_boolean(&str_expr("0")), Some(true));
  assert_eq!(to_js_boolean(&str_expr("false")), Some(true));
  assert_eq!(to_js_boolean(&num_expr(-1.0)), Some(true));
  assert_eq!(to_js_boolean(&num_expr(f64::INFINITY)), Some(true));
  assert_eq!(to_js_boolean(&bool_expr(true)), Some(true));
  assert_eq!(to_js_boolean(&ident_expr("Infinity")), Some(true));
}

#[test]
fn the_two_spellings_of_a_value_agree_on_its_boolean_and_its_nullishness() {
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
fn a_value_of_no_readable_kind_has_no_boolean() {
  assert_eq!(to_js_boolean(&ident_expr("someBinding")), None);
  assert_eq!(to_js_boolean(&this_expr()), None);
}

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

// Node oracle: `node -e "console.log(a === b)"`.
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

// Node oracle: `node -e "console.log(a == b)"`.
#[test]
fn loose_equals_applies_the_three_coercions_and_no_others() {
  let cases: [(Expr, Expr, bool); 14] = [
    (num_expr(1.0), str_expr("1"), true),
    (str_expr("1"), num_expr(1.0), true),
    (num_expr(1.0), str_expr("1px"), false),
    (num_expr(0.0), str_expr(""), true),
    (num_expr(0.0), str_expr("  "), true),
    (bool_expr(true), num_expr(1.0), true),
    (num_expr(0.0), bool_expr(false), true),
    (bool_expr(true), str_expr("1"), true),
    (null_expr(), ident_expr("undefined"), true),
    (null_expr(), num_expr(0.0), false),
    (ident_expr("undefined"), str_expr(""), false),
    (null_expr(), bool_expr(false), false),
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

  let zero_expr = num_expr(0.0);
  let (high_value, zero) = primitive_pair(&high, &zero_expr);
  assert!(!loose_equals(&high_value, &zero));
}

// Node oracle: `node -e "console.log(a < b)"`.
#[test]
fn js_less_than_orders_two_strings_by_code_unit_and_everything_else_by_number() {
  let cases: [(Expr, Expr, bool); 12] = [
    (str_expr("10"), str_expr("9"), true),
    (str_expr("2"), str_expr("10"), false),
    (str_expr("a"), str_expr("b"), true),
    (str_expr(""), str_expr("a"), true),
    (str_expr("a"), str_expr(""), false),
    (str_expr("10"), num_expr(9.0), false),
    (num_expr(9.0), str_expr("10"), true),
    (null_expr(), num_expr(1.0), true),
    (num_expr(1.0), null_expr(), false),
    (bool_expr(false), num_expr(1.0), true),
    (num_expr(1.0), bool_expr(true), false),
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

#[test]
fn to_js_default_primitive_leaves_the_default_pair_to_the_string_path() {
  assert!(to_js_default_primitive(&object_expr(vec![])).is_none());

  let answers_an_object = object_expr(vec![key_value_prop(
    ident_key("valueOf"),
    returning_arrow(object_expr(vec![])),
  )]);

  assert!(to_js_default_primitive(&answers_an_object).is_none());
}

#[track_caller]
fn primitive_pair<'a>(left: &'a Expr, right: &'a Expr) -> (Primitive<'a>, Primitive<'a>) {
  match (to_js_primitive(left), to_js_primitive(right)) {
    (Some(left), Some(right)) => (left, right),
    _ => panic!("both sides of a case have to be primitives"),
  }
}

/// A string literal containing one lone surrogate.
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

// Babel refuses RegExp conversion methods; see coercions.rs::own_conversion_method.
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
