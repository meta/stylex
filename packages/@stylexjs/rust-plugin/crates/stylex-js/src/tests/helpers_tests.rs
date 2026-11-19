// Source: crates/stylex-js/src/helpers.rs

use super::*;
use swc_core::{
  atoms::{
    Wtf8Atom,
    wtf8::{CodePoint, Wtf8Buf},
  },
  common::DUMMY_SP,
  ecma::ast::{ComputedPropName, Ident, IdentName, Number, Str, UnaryExpr},
};

fn ident_expr(name: &str) -> Expr {
  Expr::Ident(Ident::new(name.into(), DUMMY_SP, Default::default()))
}

fn member_ident(name: &str) -> MemberProp {
  MemberProp::Ident(IdentName::new(name.into(), DUMMY_SP))
}

#[test]
fn valid_callee_detection() {
  assert!(is_valid_callee(&ident_expr("Math")));
  assert!(!is_valid_callee(&ident_expr("console")));
  assert!(!is_valid_callee(&Expr::Lit(Lit::Num(Number {
    span: DUMMY_SP,
    value: 1.0,
    raw: None,
  }))));
}

#[test]
#[should_panic(expected = "The function being called must be a static identifier.")]
fn get_callee_name_panics_for_non_identifier() {
  let _ = get_callee_name(&Expr::Unary(UnaryExpr {
    span: DUMMY_SP,
    op: UnaryOp::Minus,
    arg: Box::new(ident_expr("x")),
  }));
}

#[test]
fn method_sets_detect_expected_members() {
  let assign = member_ident("assign");
  let push = member_ident("push");
  let map = member_ident("map");

  assert!(!is_valid_callee_method("Object", &assign));
  assert!(is_mutating_object_method(&assign));

  assert!(!is_valid_callee_method("Object", &push));
  assert!(!is_mutating_object_method(&push));

  assert!(!is_valid_callee_method("Object", &map));
  assert!(!is_mutating_object_method(&map));

  assert!(is_valid_callee_method("Object", &member_ident("keys")));
  assert!(is_valid_callee_method("Math", &member_ident("max")));
  assert!(!is_valid_callee_method("Math", &member_ident("keys")));
  assert!(!is_valid_callee_method("Object", &member_ident("max")));

  assert!(!is_valid_callee_method("Reflect", &member_ident("get")));
}

#[test]
fn computed_props_are_not_classified_as_identifier_methods() {
  let computed = MemberProp::Computed(ComputedPropName {
    span: DUMMY_SP,
    expr: Box::new(Expr::Lit(Lit::Str(Str {
      span: DUMMY_SP,
      value: "assign".into(),
      raw: None,
    }))),
  });

  assert!(!is_valid_callee_method("Object", &computed));
  assert!(!is_mutating_object_method(&computed));
}

#[test]
#[should_panic(expected = "The method name in a call expression must be a static identifier.")]
fn get_method_name_panics_for_non_identifier() {
  let computed = MemberProp::Computed(ComputedPropName {
    span: DUMMY_SP,
    expr: Box::new(Expr::Lit(Lit::Num(Number {
      span: DUMMY_SP,
      value: 1.0,
      raw: None,
    }))),
  });
  let _ = get_method_name(&computed);
}

#[test]
#[should_panic(expected = "String value contains invalid UTF-8 encoding.")]
fn id_prop_extraction_panics_for_invalid_utf8() {
  // A lone surrogate: valid WTF-8 storage, invalid UTF-8 decoding.
  let mut lone_surrogate = Wtf8Buf::new();

  match CodePoint::from_u32(0xd800) {
    Some(code_point) => lone_surrogate.push(code_point),
    None => panic!("U+D800 is within the code-point range."),
  }

  let computed_invalid = MemberProp::Computed(ComputedPropName {
    span: DUMMY_SP,
    expr: Box::new(Expr::Lit(Lit::Str(Str {
      span: DUMMY_SP,
      value: Wtf8Atom::new(lone_surrogate),
      raw: None,
    }))),
  });

  let _ = is_id_prop(&computed_invalid);
}
