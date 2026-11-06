//! Tests for `key_value_name`, `convert_key_value_to_str` and
//! `get_key_values_from_object`.

use crate::ast::{
  convertors::{convert_key_value_to_str, get_key_values_from_object, key_value_name},
  factories::{
    create_ident, create_key_value_prop, create_number_lit, create_object_lit, create_string_lit,
  },
};
use std::borrow::Cow;

use swc_core::{
  atoms::Atom,
  common::DUMMY_SP,
  ecma::ast::{
    BigInt, BigIntValue, ComputedPropName, Expr, Function, IdentName, KeyValueProp, Number, Prop,
    PropName, PropOrSpread, SpreadElement, Str, Tpl, TplElement,
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
fn key_ident() {
  let kv = key_value_with_name(PropName::Ident(IdentName {
    span: DUMMY_SP,
    sym: Atom::new("foo"),
  }));
  assert_eq!(convert_key_value_to_str(&kv), "foo");
}

#[test]
fn key_string_literal() {
  let kv = key_value_with_name(PropName::Str(Str {
    span: DUMMY_SP,
    value: Atom::new("a key").into(),
    raw: None,
  }));
  assert_eq!(convert_key_value_to_str(&kv), "a key");
}

#[test]
fn key_number_literal() {
  let kv = key_value_with_name(PropName::Num(Number {
    span: DUMMY_SP,
    value: 42.0,
    raw: None,
  }));
  assert_eq!(convert_key_value_to_str(&kv), "42");
}

#[test]
fn key_big_int_literal() {
  let kv = key_value_with_name(PropName::BigInt(BigInt {
    span: DUMMY_SP,
    value: Box::new(BigIntValue::from(7u32)),
    raw: None,
  }));
  assert_eq!(convert_key_value_to_str(&kv), "7");
}

#[test]
fn key_computed_string_literal() {
  let kv = key_value_with_name(PropName::Computed(ComputedPropName {
    span: DUMMY_SP,
    expr: Box::new(Expr::Lit(create_string_lit("computed"))),
  }));
  assert_eq!(convert_key_value_to_str(&kv), "computed");
}

#[test]
fn key_computed_number_literal_via_to_string() {
  // Lit::Num goes through convert_lit_to_string which serialises to text.
  let kv = key_value_with_name(PropName::Computed(ComputedPropName {
    span: DUMMY_SP,
    expr: Box::new(Expr::Lit(create_number_lit(3.5))),
  }));
  assert_eq!(convert_key_value_to_str(&kv), "3.5");
}

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
  // null literal is unsupported.
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
  // Template with an expression interpolation is not a simple literal.
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

#[test]
#[should_panic(expected = "Computed key is not a literal")]
fn key_computed_non_lit_non_tpl_panics() {
  let kv = key_value_with_name(PropName::Computed(ComputedPropName {
    span: DUMMY_SP,
    expr: Box::new(Expr::Ident(create_ident("x"))),
  }));
  let _ = convert_key_value_to_str(&kv);
}

// ---------- get_key_values_from_object ----------

#[test]
fn extracts_key_value_props() {
  let obj = create_object_lit(vec![
    create_key_value_prop("a", Expr::Lit(create_string_lit("1"))),
    create_key_value_prop("b", Expr::Lit(create_number_lit(2.0))),
  ]);
  let result = get_key_values_from_object(&obj);
  assert_eq!(result.len(), 2);
}

#[test]
fn expands_shorthand_props() {
  // Shorthand prop: { foo } → key:foo, value:foo (ident).
  let prop = PropOrSpread::Prop(Box::new(Prop::Shorthand(create_ident("foo"))));
  let obj = create_object_lit(vec![prop]);
  let result = get_key_values_from_object(&obj);
  assert_eq!(result.len(), 1);
  // After expansion the prop becomes a key/value with both sides "foo".
  match &result[0].key {
    PropName::Ident(ident) => assert_eq!(ident.sym.as_ref(), "foo"),
    _ => panic!("expected expanded ident name"),
  }
}

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
  // Use a getter to produce a non key-value, non-shorthand prop.
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

#[test]
fn empty_object_returns_empty_vec() {
  let obj = create_object_lit(vec![]);
  assert!(get_key_values_from_object(&obj).is_empty());
}

// ---------- key_value_name ----------

/// A key that already holds its name is borrowed, not copied.
///
/// This is the whole reason the reader answers a `Cow`: an identifier and a
/// string hold their own text, and a lookup that compares every property of an
/// object copied each of them to read it. The text itself is pinned by the
/// cases above, which the owned form and this one both answer through.
#[test]
fn a_key_that_holds_its_name_is_borrowed() {
  let ident = key_value_with_name(PropName::Ident(IdentName {
    span: DUMMY_SP,
    sym: Atom::new("foo"),
  }));
  let string = key_value_with_name(PropName::Str(Str {
    span: DUMMY_SP,
    value: Atom::new("a key").into(),
    raw: None,
  }));

  assert!(matches!(key_value_name(&ident), Cow::Borrowed(_)));
  assert!(matches!(key_value_name(&string), Cow::Borrowed(_)));
}

/// The three shapes that spell a name the key does not hold own what they
/// build, so nothing borrows from a value that goes out of scope.
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
