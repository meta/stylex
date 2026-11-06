//! Tests for the object and member key readers.

use crate::ast::keys::{
  named_key_value, namespace_name_from_member_prop, namespace_name_from_prop_key,
  prop_as_key_value, try_namespace_name_from_prop_key,
};
use swc_core::{
  atoms::Atom,
  common::DUMMY_SP,
  ecma::ast::{
    BigInt, BigIntValue, ComputedPropName, Expr, Function, GetterProp, Ident, IdentName,
    KeyValueProp, Lit, MemberProp, MethodProp, Number, PrivateName, Prop, PropName, PropOrSpread,
    SpreadElement, Str, Tpl, TplElement,
  },
};

fn ident_name(name: &str) -> IdentName {
  IdentName {
    span: DUMMY_SP,
    sym: Atom::new(name),
  }
}

fn string_lit(value: &str) -> Str {
  Str {
    span: DUMMY_SP,
    value: Atom::new(value).into(),
    raw: None,
  }
}

fn number_lit(value: f64) -> Number {
  Number {
    span: DUMMY_SP,
    value,
    raw: None,
  }
}

fn big_int_lit(value: i64) -> BigInt {
  BigInt {
    span: DUMMY_SP,
    value: Box::new(BigIntValue::from(value)),
    raw: None,
  }
}

fn computed(expr: Expr) -> ComputedPropName {
  ComputedPropName {
    span: DUMMY_SP,
    expr: Box::new(expr),
  }
}

/// A template literal with no expressions.
fn static_tpl(text: &str) -> Tpl {
  Tpl {
    span: DUMMY_SP,
    exprs: vec![],
    quasis: vec![TplElement {
      span: DUMMY_SP,
      tail: true,
      cooked: Some(Atom::new(text).into()),
      raw: Atom::new(text),
    }],
  }
}

/// A template literal with an interpolation.
fn dynamic_tpl() -> Tpl {
  Tpl {
    span: DUMMY_SP,
    exprs: vec![Box::new(Expr::Ident(Ident::new_no_ctxt(
      Atom::new("value"),
      DUMMY_SP,
    )))],
    quasis: vec![
      TplElement {
        span: DUMMY_SP,
        tail: false,
        cooked: Some(Atom::new("a").into()),
        raw: Atom::new("a"),
      },
      TplElement {
        span: DUMMY_SP,
        tail: true,
        cooked: Some(Atom::new("b").into()),
        raw: Atom::new("b"),
      },
    ],
  }
}

fn key_value_prop(key: PropName, value: Expr) -> PropOrSpread {
  PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
    key,
    value: Box::new(value),
  })))
}

// ---------- namespace_name_from_prop_key ----------

// ---------- try_namespace_name_from_prop_key ----------

/// Both readers agree on every shape with a readable name.
#[test]
fn both_readers_agree_on_every_readable_shape() {
  let keys = [
    PropName::Ident(ident_name("root")),
    PropName::Str(string_lit("a key")),
    PropName::Num(number_lit(1e21)),
    PropName::BigInt(big_int_lit(12)),
    PropName::Computed(computed(Expr::Lit(Lit::Str(string_lit("root"))))),
    PropName::Computed(computed(Expr::Lit(Lit::Num(number_lit(7.0))))),
    PropName::Computed(computed(Expr::Lit(Lit::BigInt(big_int_lit(12))))),
    PropName::Computed(computed(Expr::Tpl(static_tpl("root")))),
    PropName::Computed(computed(Expr::Tpl(dynamic_tpl()))),
    PropName::Computed(computed(Expr::Lit(Lit::Null(swc_core::ecma::ast::Null {
      span: DUMMY_SP,
    })))),
    PropName::Computed(computed(Expr::Ident(Ident::new_no_ctxt(
      Atom::new("name"),
      DUMMY_SP,
    )))),
  ];

  for key in keys {
    assert_eq!(
      try_namespace_name_from_prop_key(&key),
      namespace_name_from_prop_key(&key),
      "the two readers disagree on {key:?}"
    );
  }
}

// ---------- namespace_name_from_member_prop ----------

#[test]
fn refuses_a_private_member() {
  let prop = MemberProp::PrivateName(PrivateName {
    span: DUMMY_SP,
    name: Atom::new("secret"),
  });

  assert_eq!(namespace_name_from_member_prop(&prop), None);
}

// ---------- prop_as_key_value ----------

#[test]
fn refuses_a_getter() {
  let prop = PropOrSpread::Prop(Box::new(Prop::Getter(GetterProp {
    span: DUMMY_SP,
    key: PropName::Ident(ident_name("root")),
    function: Box::new(Function::default()),
  })));

  assert!(prop_as_key_value(&prop).is_none());
}

// ---------- named_key_value ----------

#[test]
#[should_panic(expected = "The spread operator (...) is not supported in this context.")]
fn refuses_a_spread_it_cannot_name() {
  let prop = PropOrSpread::Spread(SpreadElement {
    dot3_token: DUMMY_SP,
    expr: Box::new(Expr::Ident(Ident::new_no_ctxt(Atom::new("rest"), DUMMY_SP))),
  });

  named_key_value(&prop);
}

#[test]
#[should_panic(expected = "A style value can only contain an array, string or number.")]
fn refuses_a_property_that_carries_no_value() {
  let prop = PropOrSpread::Prop(Box::new(Prop::Method(MethodProp {
    key: PropName::Ident(ident_name("root")),
    function: Box::default(),
  })));

  named_key_value(&prop);
}

#[test]
#[should_panic(expected = "The key has no name at compile time.")]
fn refuses_a_key_with_no_name() {
  let prop = key_value_prop(
    PropName::Computed(ComputedPropName {
      span: DUMMY_SP,
      expr: Box::new(Expr::Ident(Ident::new_no_ctxt(
        Atom::new("runtime"),
        DUMMY_SP,
      ))),
    }),
    Expr::Lit(Lit::Num(number_lit(1.0))),
  );

  named_key_value(&prop);
}

// ---------- collect_object_lit_keys ----------
