//! Tests for the object and member key readers.

use crate::ast::keys::{
  collect_object_lit_keys, named_key_value, namespace_name_from_member_prop,
  namespace_name_from_prop_key, prop_as_key_value, try_namespace_name_from_prop_key,
};
use swc_core::{
  atoms::{Atom, Wtf8Atom},
  common::DUMMY_SP,
  ecma::ast::{
    ArrowExpr, ArrowFunctionBody, BigInt, BigIntValue, BindingIdent, ComputedPropName, Expr,
    Function, GetterProp, Ident, IdentName, KeyValueProp, Lit, MemberProp, MethodProp, Number,
    ObjectLit, Pat, PrivateName, Prop, PropName, PropOrSpread, SpreadElement, Str, Tpl, TplElement,
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

/// A template literal with no expressions, which reads back as its one quasi.
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

/// A template literal with an interpolation, so nothing static can be read off
/// it.
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

fn object_of(props: Vec<PropOrSpread>) -> ObjectLit {
  ObjectLit {
    span: DUMMY_SP,
    props,
  }
}

// ---------- namespace_name_from_prop_key ----------

#[test]
fn reads_an_identifier_key() {
  let key = PropName::Ident(ident_name("root"));

  assert_eq!(namespace_name_from_prop_key(&key), Some(Atom::new("root")));
}

#[test]
fn reads_a_string_key() {
  let key = PropName::Str(string_lit("a key"));

  assert_eq!(namespace_name_from_prop_key(&key), Some(Atom::new("a key")));
}

#[test]
fn reads_an_integer_number_key() {
  let key = PropName::Num(number_lit(42.0));

  assert_eq!(namespace_name_from_prop_key(&key), Some(Atom::new("42")));
}

/// `1e21` is where the two spellings part: JavaScript prints `1e+21`, Rust
/// prints the twenty-two digits. The key names a property, so the spelling
/// reaches the stylesheet and has to be the language's.
#[test]
fn reads_a_number_key_the_way_javascript_spells_it() {
  let key = PropName::Num(number_lit(1e21));

  assert_eq!(namespace_name_from_prop_key(&key), Some(Atom::new("1e+21")));
}

/// The same number written as a computed key reads the same way, because it
/// names the same property.
#[test]
fn reads_a_computed_number_key_the_way_javascript_spells_it() {
  let key = PropName::Computed(computed(Expr::Lit(Lit::Num(number_lit(1e21)))));

  assert_eq!(namespace_name_from_prop_key(&key), Some(Atom::new("1e+21")));
}

/// A member is read by the same literal reader, so a computed member of that
/// number gets the JavaScript spelling too.
#[test]
fn reads_a_computed_number_member_the_way_javascript_spells_it() {
  let prop = MemberProp::Computed(computed(Expr::Lit(Lit::Num(number_lit(1e21)))));

  assert_eq!(
    namespace_name_from_member_prop(&prop),
    Some(Atom::new("1e+21"))
  );
}

/// The far ends of the range, where a spelling mistake is most visible: small
/// magnitudes take an exponent, `-0` loses its sign as `String(-0)` does, and
/// the three non-finite values keep their names.
///
/// No JavaScript source spells a key `NaN` or `Infinity` as a number literal,
/// so these reach the reader only from a tree the compiler builds itself. They
/// are asserted anyway, to hold the reader to one spelling for every `f64`.
#[test]
fn spells_every_number_key_the_way_javascript_does() {
  let cases = [
    (1e-7, "1e-7"),
    (-0.0, "0"),
    (f64::MAX, "1.7976931348623157e+308"),
    (f64::MIN_POSITIVE, "2.2250738585072014e-308"),
    (f64::NAN, "NaN"),
    (f64::INFINITY, "Infinity"),
    (f64::NEG_INFINITY, "-Infinity"),
  ];

  for (value, spelling) in cases {
    let key = PropName::Num(number_lit(value));

    assert_eq!(
      namespace_name_from_prop_key(&key),
      Some(Atom::new(spelling)),
      "number key {value} must spell as {spelling}"
    );
  }
}

#[test]
fn reads_a_big_int_key() {
  let key = PropName::BigInt(big_int_lit(9_007_199_254_740_993));

  assert_eq!(
    namespace_name_from_prop_key(&key),
    Some(Atom::new("9007199254740993"))
  );
}

#[test]
fn reads_a_computed_string_key() {
  let key = PropName::Computed(computed(Expr::Lit(Lit::Str(string_lit("root")))));

  assert_eq!(namespace_name_from_prop_key(&key), Some(Atom::new("root")));
}

#[test]
fn reads_a_computed_number_key() {
  let key = PropName::Computed(computed(Expr::Lit(Lit::Num(number_lit(7.0)))));

  assert_eq!(namespace_name_from_prop_key(&key), Some(Atom::new("7")));
}

#[test]
fn reads_a_computed_big_int_key() {
  let key = PropName::Computed(computed(Expr::Lit(Lit::BigInt(big_int_lit(12)))));

  assert_eq!(namespace_name_from_prop_key(&key), Some(Atom::new("12")));
}

#[test]
fn reads_a_computed_static_template_key() {
  let key = PropName::Computed(computed(Expr::Tpl(static_tpl("root"))));

  assert_eq!(namespace_name_from_prop_key(&key), Some(Atom::new("root")));
}

#[test]
fn refuses_a_computed_template_key_with_an_interpolation() {
  let key = PropName::Computed(computed(Expr::Tpl(dynamic_tpl())));

  assert_eq!(namespace_name_from_prop_key(&key), None);
}

#[test]
fn refuses_a_computed_key_whose_literal_has_no_name() {
  let key = PropName::Computed(computed(Expr::Lit(Lit::Null(swc_core::ecma::ast::Null {
    span: DUMMY_SP,
  }))));

  assert_eq!(namespace_name_from_prop_key(&key), None);
}

#[test]
fn refuses_a_computed_key_that_is_not_a_literal() {
  let key = PropName::Computed(computed(Expr::Ident(Ident::new_no_ctxt(
    Atom::new("name"),
    DUMMY_SP,
  ))));

  assert_eq!(namespace_name_from_prop_key(&key), None);
}

// ---------- try_namespace_name_from_prop_key ----------

/// Storage that is valid WTF-8 but not valid UTF-8: a lone surrogate, which a
/// JavaScript source can spell as `"\ud800"`.
fn lossy_wtf8_atom() -> Wtf8Atom {
  unsafe { Wtf8Atom::from_bytes_unchecked(&[0xed, 0xa0, 0x80]) }
}

fn lossy_string_lit() -> Str {
  Str {
    span: DUMMY_SP,
    value: lossy_wtf8_atom(),
    raw: None,
  }
}

/// The two readers must answer alike on every shape that carries a readable
/// name, so a caller can pick either one on the strength of what it wants done
/// with unreadable text alone.
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

#[test]
fn skips_a_string_key_that_cannot_be_read() {
  let key = PropName::Str(lossy_string_lit());

  assert_eq!(try_namespace_name_from_prop_key(&key), None);
}

#[test]
fn skips_a_computed_string_key_that_cannot_be_read() {
  let key = PropName::Computed(computed(Expr::Lit(Lit::Str(lossy_string_lit()))));

  assert_eq!(try_namespace_name_from_prop_key(&key), None);
}

/// A template element keeps its cooked text as the same storage a string
/// literal does, so the same unreadable text reaches the reader through it.
#[test]
fn skips_a_computed_template_key_that_cannot_be_read() {
  let tpl = Tpl {
    span: DUMMY_SP,
    exprs: vec![],
    quasis: vec![TplElement {
      span: DUMMY_SP,
      tail: true,
      cooked: Some(lossy_wtf8_atom()),
      raw: Atom::new(""),
    }],
  };

  assert_eq!(
    try_namespace_name_from_prop_key(&PropName::Computed(computed(Expr::Tpl(tpl)))),
    None
  );
}

/// A template element with no cooked text spells an invalid escape sequence.
/// It names no property, and the reader says so rather than refusing.
#[test]
fn skips_a_computed_template_key_with_no_cooked_text() {
  let tpl = Tpl {
    span: DUMMY_SP,
    exprs: vec![],
    quasis: vec![TplElement {
      span: DUMMY_SP,
      tail: true,
      cooked: None,
      raw: Atom::new("\\u{}"),
    }],
  };

  assert_eq!(
    try_namespace_name_from_prop_key(&PropName::Computed(computed(Expr::Tpl(tpl)))),
    None
  );
}

// ---------- namespace_name_from_member_prop ----------

#[test]
fn reads_an_identifier_member() {
  let prop = MemberProp::Ident(ident_name("color"));

  assert_eq!(
    namespace_name_from_member_prop(&prop),
    Some(Atom::new("color"))
  );
}

#[test]
fn reads_a_computed_member() {
  let prop = MemberProp::Computed(computed(Expr::Lit(Lit::Str(string_lit("color")))));

  assert_eq!(
    namespace_name_from_member_prop(&prop),
    Some(Atom::new("color"))
  );
}

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
fn accepts_a_key_value_property() {
  let prop = key_value_prop(
    PropName::Ident(ident_name("root")),
    Expr::Lit(Lit::Num(number_lit(1.0))),
  );

  assert!(prop_as_key_value(&prop).is_some());
}

#[test]
fn refuses_a_spread() {
  let prop = PropOrSpread::Spread(SpreadElement {
    dot3_token: DUMMY_SP,
    expr: Box::new(Expr::Ident(Ident::new_no_ctxt(Atom::new("rest"), DUMMY_SP))),
  });

  assert!(prop_as_key_value(&prop).is_none());
}

#[test]
fn refuses_a_shorthand() {
  let prop = PropOrSpread::Prop(Box::new(Prop::Shorthand(Ident::new_no_ctxt(
    Atom::new("root"),
    DUMMY_SP,
  ))));

  assert!(prop_as_key_value(&prop).is_none());
}

#[test]
fn refuses_a_getter() {
  let prop = PropOrSpread::Prop(Box::new(Prop::Getter(GetterProp {
    span: DUMMY_SP,
    key: PropName::Ident(ident_name("root")),
    function: Box::new(Function::default()),
  })));

  assert!(prop_as_key_value(&prop).is_none());
}

#[test]
fn refuses_a_method() {
  let prop = PropOrSpread::Prop(Box::new(Prop::Method(MethodProp {
    key: PropName::Ident(ident_name("root")),
    function: Box::default(),
  })));

  assert!(prop_as_key_value(&prop).is_none());
}

// ---------- named_key_value ----------

/// The name and the property come back together, whichever shape the key is
/// written in.
#[test]
fn reads_the_name_and_the_property_together() {
  let prop = key_value_prop(
    PropName::Str(string_lit("--color")),
    Expr::Lit(Lit::Num(number_lit(1.0))),
  );

  let (name, key_value) = named_key_value(&prop);

  assert_eq!(name, Atom::new("--color"));
  assert!(matches!(
    key_value.value.as_ref(),
    Expr::Lit(Lit::Num(number)) if number.value == 1.0
  ));
}

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

#[test]
fn collects_nothing_from_an_empty_object() {
  let object = object_of(vec![]);

  assert!(collect_object_lit_keys(&object).next().is_none());
}

#[test]
fn collects_keys_in_source_order() {
  let object = object_of(vec![
    key_value_prop(
      PropName::Ident(ident_name("root")),
      Expr::Lit(Lit::Num(number_lit(1.0))),
    ),
    key_value_prop(
      PropName::Str(string_lit("nested")),
      Expr::Lit(Lit::Num(number_lit(2.0))),
    ),
  ]);

  assert_eq!(
    collect_object_lit_keys(&object).collect::<Vec<_>>(),
    vec![Atom::new("root"), Atom::new("nested")]
  );
}

#[test]
fn counts_a_key_written_twice_as_two_properties() {
  let object = object_of(vec![
    key_value_prop(
      PropName::Ident(ident_name("root")),
      Expr::Lit(Lit::Num(number_lit(1.0))),
    ),
    key_value_prop(
      PropName::Ident(ident_name("root")),
      Expr::Lit(Lit::Num(number_lit(2.0))),
    ),
  ]);

  assert_eq!(
    collect_object_lit_keys(&object).collect::<Vec<_>>(),
    vec![Atom::new("root"), Atom::new("root")]
  );
}

#[test]
fn skips_properties_that_have_no_readable_key() {
  let object = object_of(vec![
    PropOrSpread::Spread(SpreadElement {
      dot3_token: DUMMY_SP,
      expr: Box::new(Expr::Ident(Ident::new_no_ctxt(Atom::new("rest"), DUMMY_SP))),
    }),
    key_value_prop(
      PropName::Computed(computed(Expr::Ident(Ident::new_no_ctxt(
        Atom::new("dynamic"),
        DUMMY_SP,
      )))),
      Expr::Lit(Lit::Num(number_lit(1.0))),
    ),
    key_value_prop(
      PropName::Ident(ident_name("root")),
      Expr::Lit(Lit::Num(number_lit(2.0))),
    ),
  ]);

  assert_eq!(
    collect_object_lit_keys(&object).collect::<Vec<_>>(),
    vec![Atom::new("root")]
  );
}

/// A large object, to show the readers stay linear and do not overflow a
/// recursion limit on width.
#[test]
fn collects_every_key_of_a_very_wide_object() {
  const WIDTH: usize = 10_000;

  let object = object_of(
    (0..WIDTH)
      .map(|index| {
        key_value_prop(
          PropName::Num(number_lit(index as f64)),
          Expr::Lit(Lit::Num(number_lit(index as f64))),
        )
      })
      .collect(),
  );

  let keys = collect_object_lit_keys(&object).collect::<Vec<_>>();

  assert_eq!(keys.len(), WIDTH);
  assert_eq!(keys.first(), Some(&Atom::new("0")));
  assert_eq!(keys.last(), Some(&Atom::new("9999")));
}

/// An arrow value carries no key of its own, so the property is read by its key
/// alone.
#[test]
fn reads_a_key_whose_value_is_a_function() {
  let object = object_of(vec![key_value_prop(
    PropName::Ident(ident_name("dynamic")),
    Expr::Arrow(ArrowExpr {
      span: DUMMY_SP,
      params: vec![Pat::Ident(BindingIdent {
        id: Ident::new_no_ctxt(Atom::new("value"), DUMMY_SP),
        type_ann: None,
      })],
      body: Box::new(ArrowFunctionBody::Expr(Box::new(Expr::Ident(
        Ident::new_no_ctxt(Atom::new("value"), DUMMY_SP),
      )))),
      is_async: false,
      is_generator: false,
      type_params: None,
      return_type: None,
      ctxt: Default::default(),
    }),
  )]);

  assert_eq!(
    collect_object_lit_keys(&object).collect::<Vec<_>>(),
    vec![Atom::new("dynamic")]
  );
}
