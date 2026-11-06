//! Tests for `key_value_name`, `convert_key_value_to_str` and
//! `get_key_values_from_object`.

use crate::ast::{
  convertors::{convert_key_value_to_str, get_key_values_from_object, key_value_name},
  factories::{create_ident, create_object_lit, create_string_lit},
};
use std::borrow::Cow;

use swc_core::{
  atoms::Atom,
  common::DUMMY_SP,
  ecma::ast::{
    BigInt, BigIntValue, ComputedPropName, Expr, Function, IdentName, KeyValueProp, Number, Prop,
    PropName, PropOrSpread, SpreadElement, Tpl, TplElement,
  },
};

fn key_value_with_name(key: PropName) -> KeyValueProp {
  KeyValueProp {
    key,
    value: Box::new(Expr::Lit(create_string_lit("ignored"))),
  }
}

// ---------- convert_key_value_to_str ----------

#[test]
fn key_computed_simple_template_literal() {
  let tpl = Tpl {
    span: DUMMY_SP,
    exprs: vec![],
    quasis: vec![TplElement {
      span: DUMMY_SP,
      tail: true,
      cooked: Some(Atom::new("tpl-key").into()),
      raw: Atom::new("tpl-key"),
    }],
  };
  let kv = key_value_with_name(PropName::Computed(ComputedPropName {
    span: DUMMY_SP,
    expr: Box::new(Expr::Tpl(tpl)),
  }));
  assert_eq!(convert_key_value_to_str(&kv), "tpl-key");
}

#[test]
#[should_panic(expected = "Computed property key must be a string or number literal")]
fn key_computed_non_string_literal_panics() {
  let kv = key_value_with_name(PropName::Computed(ComputedPropName {
    span: DUMMY_SP,
    expr: Box::new(Expr::Lit(swc_core::ecma::ast::Lit::Null(
      swc_core::ecma::ast::Null { span: DUMMY_SP },
    ))),
  }));
  let _ = convert_key_value_to_str(&kv);
}

#[test]
#[should_panic(expected = "Computed key is not a literal")]
fn key_computed_complex_template_panics() {
  let tpl = Tpl {
    span: DUMMY_SP,
    exprs: vec![Box::new(Expr::Ident(create_ident("x")))],
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
  };
  let kv = key_value_with_name(PropName::Computed(ComputedPropName {
    span: DUMMY_SP,
    expr: Box::new(Expr::Tpl(tpl)),
  }));
  let _ = convert_key_value_to_str(&kv);
}

// ---------- get_key_values_from_object ----------

#[test]
#[should_panic(expected = "spread operator")]
fn spread_prop_panics() {
  let obj = create_object_lit(vec![PropOrSpread::Spread(SpreadElement {
    dot3_token: DUMMY_SP,
    expr: Box::new(Expr::Ident(create_ident("rest"))),
  })]);
  let _ = get_key_values_from_object(&obj);
}

#[test]
#[should_panic(expected = "style value")]
fn non_key_value_prop_panics() {
  // A getter is neither key-value nor shorthand.
  let getter = Prop::Getter(swc_core::ecma::ast::GetterProp {
    span: DUMMY_SP,
    key: PropName::Ident(IdentName {
      span: DUMMY_SP,
      sym: Atom::new("g"),
    }),
    function: Box::new(Function::default()),
  });
  let obj = create_object_lit(vec![PropOrSpread::Prop(Box::new(getter))]);
  let _ = get_key_values_from_object(&obj);
}

// ---------- key_value_name ----------

/// Names the key doesn't hold are owned.
#[test]
fn a_key_that_spells_its_name_is_owned() {
  let shapes = [
    PropName::Num(Number {
      span: DUMMY_SP,
      value: 42.0,
      raw: None,
    }),
    PropName::BigInt(BigInt {
      span: DUMMY_SP,
      value: Box::new(BigIntValue::from(100u32)),
      raw: None,
    }),
    PropName::Computed(ComputedPropName {
      span: DUMMY_SP,
      expr: Box::new(Expr::Lit(create_string_lit("computed"))),
    }),
  ];

  for shape in shapes {
    let kv = key_value_with_name(shape);

    assert!(matches!(key_value_name(&kv), Cow::Owned(_)));
  }
}
