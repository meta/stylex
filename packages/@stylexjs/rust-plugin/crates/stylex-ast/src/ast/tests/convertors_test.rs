//! Tests for AST convertor functions that transform between node types.

use crate::ast::{convertors::*, factories::*};
use swc_core::{
  atoms::{Wtf8Atom, wtf8::Wtf8Buf},
  common::DUMMY_SP,
  ecma::ast::*,
};

/// The error names the literal kind it could not read.
#[test]
fn convert_lit_to_number_names_the_literal_kind_it_refused() {
  let cases = [
    (
      Lit::Null(swc_core::ecma::ast::Null { span: DUMMY_SP }),
      "NullLiteral",
    ),
    (
      Lit::Regex(Regex {
        span: DUMMY_SP,
        exp: "a".into(),
        flags: "".into(),
      }),
      "RegExpLiteral",
    ),
    (
      Lit::JSXText(JSXText {
        span: DUMMY_SP,
        value: "t".into(),
        raw: "t".into(),
      }),
      "JSXText",
    ),
  ];

  for (lit, kind) in cases {
    match convert_lit_to_number(&lit) {
      Ok(number) => panic!("expected {:?} to refuse, got {}", lit, number),
      Err(error) => assert_eq!(
        error.to_string(),
        format!("Value is not a number: {}", kind),
        "wrong label for {:?}",
        lit
      ),
    }
  }
}

/// Half of an astral character answers U+FFFD, from both well-formed and lone-surrogate text;
/// `None` means only past the end.
#[test]
fn atom_utf16_char_at_replaces_half_of_a_character() {
  let paired = Wtf8Atom::from("\u{1F600}a");

  assert_eq!(
    atom_utf16_char_at(&paired, 0),
    Some(char::REPLACEMENT_CHARACTER)
  );
  assert_eq!(
    atom_utf16_char_at(&paired, 1),
    Some(char::REPLACEMENT_CHARACTER)
  );
  assert_eq!(atom_utf16_char_at(&paired, 2), Some('a'));

  let lone = Wtf8Atom::from(Wtf8Buf::from_ill_formed_utf16(&[0x0061, 0xD83D, 0x0062]));

  assert!(
    lone.as_str().is_none(),
    "expected an atom with no UTF-8 form to test against"
  );
  assert_eq!(atom_utf16_char_at(&lone, 0), Some('a'));
  assert_eq!(
    atom_utf16_char_at(&lone, 1),
    Some(char::REPLACEMENT_CHARACTER)
  );
  assert_eq!(atom_utf16_char_at(&lone, 2), Some('b'));
  assert_eq!(atom_utf16_char_at(&lone, 3), None);
}

#[test]
fn extract_tpl_cooked_value_valid() {
  let elem = TplElement {
    span: DUMMY_SP,
    tail: true,
    cooked: Some("cooked".into()),
    raw: "cooked".into(),
  };
  assert_eq!(extract_tpl_cooked_value(&elem), "cooked");
}

#[test]
fn convert_lit_to_string_bigint() {
  let big = BigInt {
    span: DUMMY_SP,
    value: Box::new(99i64.into()),
    raw: None,
  };
  let lit = Lit::BigInt(big);
  assert_eq!(convert_lit_to_string(&lit), Some("99".to_string()));
}

#[test]
fn extract_str_lit_ref_num_returns_none() {
  let lit = create_number_lit(1.0);
  assert!(extract_str_lit_ref(&lit).is_none());
}

#[test]
fn expand_shorthand_prop_converts() {
  let ident = Ident::from("x");
  let mut prop = Box::new(Prop::Shorthand(ident));
  expand_shorthand_prop(&mut prop);
  assert!(matches!(*prop, Prop::KeyValue(_)));
}

fn computed_bigint_member_prop(value: i64) -> MemberProp {
  create_computed_member_prop(Expr::Lit(Lit::BigInt(BigInt {
    span: DUMMY_SP,
    value: Box::new(value.into()),
    raw: None,
  })))
}

#[test]
fn convert_member_prop_to_string_bigint() {
  // A computed BigInt key (`obj[123n]`) renders as its decimal digits.
  assert_eq!(
    convert_member_prop_to_string(&computed_bigint_member_prop(123)).as_deref(),
    Some("123")
  );
  assert_eq!(
    convert_member_prop_to_string(&computed_bigint_member_prop(-45)).as_deref(),
    Some("-45")
  );
}

#[test]
fn normalize_expr_handles_deeply_nested_parens_without_recursion() {
  let mut expr = create_string_expr("before");
  for _ in 0..16_384 {
    expr = Expr::Paren(ParenExpr {
      span: DUMMY_SP,
      expr: Box::new(expr),
    });
  }

  assert!(matches!(normalize_expr(&expr), Expr::Lit(Lit::Str(_))));
  *normalize_expr_mut(&mut expr) = create_string_expr("after");

  // Drop iteratively so cleanup does not recurse.
  while let Expr::Paren(paren) = expr {
    expr = *paren.expr;
  }
  assert!(matches!(expr, Expr::Lit(Lit::Str(_))));
}

#[test]
#[should_panic(expected = "Variable declaration must be initialized")]
fn get_expr_from_var_decl_panics_without_initializer() {
  let decl = VarDeclarator {
    span: DUMMY_SP,
    name: Pat::Ident(create_binding_ident(create_ident("a"))),
    init: None,
    definite: false,
  };
  get_expr_from_var_decl(&decl);
}

// is_js_undefined
