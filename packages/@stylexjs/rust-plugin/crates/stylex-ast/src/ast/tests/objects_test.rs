use swc_core::{
  common::DUMMY_SP,
  ecma::ast::{
    BigInt, ComputedPropName, Expr, Function, GetterProp, IdentName, KeyValueProp, Number,
    ObjectLit, Prop, PropName, PropOrSpread, SpreadElement, Str,
  },
};

use crate::ast::convertors::{convert_wtf8_to_atom, create_number_expr};
use crate::ast::factories::create_ident;
use crate::ast::objects::{assign_props, order_own_keys, remove_duplicates};

fn make_kv_prop(key: &str, val: f64) -> PropOrSpread {
  PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
    key: PropName::Ident(IdentName {
      span: DUMMY_SP,
      sym: key.into(),
    }),
    value: Box::new(create_number_expr(val)),
  })))
}

fn make_kv_str_key_prop(key: &str, val: f64) -> PropOrSpread {
  PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
    key: PropName::Str(Str {
      span: DUMMY_SP,
      value: key.into(),
      raw: None,
    }),
    value: Box::new(create_number_expr(val)),
  })))
}

fn make_shorthand_prop(name: &str) -> PropOrSpread {
  PropOrSpread::Prop(Box::new(Prop::Shorthand(create_ident(name))))
}

fn make_spread_prop() -> PropOrSpread {
  PropOrSpread::Spread(SpreadElement {
    dot3_token: DUMMY_SP,
    expr: Box::new(create_number_expr(1.0)),
  })
}

fn make_kv_num_key_obj_prop(key: f64, inner: Vec<PropOrSpread>) -> PropOrSpread {
  PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
    key: PropName::Num(Number {
      span: DUMMY_SP,
      value: key,
      raw: None,
    }),
    value: Box::new(Expr::Object(ObjectLit {
      span: DUMMY_SP,
      props: inner,
    })),
  })))
}

fn make_bigint_obj_prop(val: u32, inner: Vec<PropOrSpread>) -> PropOrSpread {
  PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
    key: PropName::BigInt(BigInt {
      span: DUMMY_SP,
      value: Box::new(val.into()),
      raw: None,
    }),
    value: Box::new(Expr::Object(ObjectLit {
      span: DUMMY_SP,
      props: inner,
    })),
  })))
}

fn make_kv_num_key_prop(key: f64, val: f64) -> PropOrSpread {
  PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
    key: PropName::Num(Number {
      span: DUMMY_SP,
      value: key,
      raw: None,
    }),
    value: Box::new(create_number_expr(val)),
  })))
}

fn make_getter_prop(key: &str) -> PropOrSpread {
  PropOrSpread::Prop(Box::new(Prop::Getter(GetterProp {
    span: DUMMY_SP,
    key: PropName::Ident(IdentName {
      span: DUMMY_SP,
      sym: key.into(),
    }),
    function: Box::new(Function::default()),
  })))
}

/// A computed-key property `{ [key]: val }`; readers never name it.
fn make_computed_key_prop(key: f64, val: f64) -> PropOrSpread {
  PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
    key: PropName::Computed(ComputedPropName {
      span: DUMMY_SP,
      expr: Box::new(create_number_expr(key)),
    }),
    value: Box::new(create_number_expr(val)),
  })))
}

fn keys_of(props: &[PropOrSpread]) -> Vec<String> {
  props
    .iter()
    .map(|prop| match prop {
      PropOrSpread::Prop(prop) => match prop.as_ref() {
        Prop::KeyValue(kv) => match &kv.key {
          PropName::Ident(ident) => ident.sym.to_string(),
          PropName::Str(strng) => convert_wtf8_to_atom(&strng.value).to_string(),
          PropName::Num(num) => num.value.to_string(),
          PropName::BigInt(big_int) => big_int.value.to_string(),
          PropName::Computed(_) => String::from("<computed>"),
        },
        Prop::Shorthand(ident) => ident.sym.to_string(),
        _ => String::from("<other>"),
      },
      PropOrSpread::Spread(_) => String::from("<spread>"),
    })
    .collect()
}

mod remove_duplicates_tests {
  use super::*;

  #[test]
  fn a_shorthand_and_a_key_value_under_one_name_are_one_property() {
    let props = vec![make_shorthand_prop("x"), make_kv_prop("x", 5.0)];

    assert_eq!(keys_of(&remove_duplicates(props)), vec!["x"]);
  }

  // `['a']` would name `a` in JS, but unevaluated it must not collide with `a`.
  #[test]
  fn a_computed_key_does_not_collide_with_the_name_it_would_spell() {
    let props = vec![
      make_kv_num_key_prop(0.0, 1.0),
      make_computed_key_prop(0.0, 2.0),
    ];

    assert_eq!(keys_of(&remove_duplicates(props)), vec!["0"]);
  }
}

mod assign_props_tests {
  use super::*;

  #[test]
  fn a_repeated_big_integer_key_is_one_key() {
    let old = vec![make_bigint_obj_prop(42, vec![make_kv_prop("x", 1.0)])];
    let new = vec![make_bigint_obj_prop(42, vec![make_kv_prop("y", 2.0)])];

    // `{ 42n: x }` names `"42"`.
    assert_eq!(keys_of(&assign_props(old, new)), vec!["42"]);
  }

  // Unnamed properties collide with nothing and keep their place.

  #[test]
  fn a_merge_of_nothing_readable_keeps_every_property() {
    let old = vec![make_computed_key_prop(0.0, 1.0), make_spread_prop()];
    let new = vec![make_getter_prop("val"), make_computed_key_prop(1.0, 2.0)];

    assert_eq!(
      keys_of(&assign_props(old, new)),
      vec!["<computed>", "<spread>", "<other>", "<computed>"]
    );
  }
}

mod order_own_keys_tests {
  use super::*;

  #[test]
  fn reads_a_numeric_key_as_the_index_it_spells() {
    let props = vec![
      make_kv_prop("color", 1.0),
      make_kv_num_key_obj_prop(3.0, vec![]),
      make_kv_str_key_prop("1", 2.0),
    ];

    assert_eq!(keys_of(&order_own_keys(props)), vec!["1", "3", "color"]);
  }

  #[test]
  fn a_property_with_no_readable_key_keeps_its_place_among_the_string_keys() {
    let props = vec![
      make_kv_prop("a", 1.0),
      make_spread_prop(),
      make_kv_str_key_prop("0", 2.0),
    ];

    assert_eq!(keys_of(&order_own_keys(props)), vec!["0", "a", "<spread>"]);
  }

  #[test]
  fn a_computed_key_reads_as_no_key_at_all() {
    let props = vec![
      make_computed_key_prop(0.0, 1.0),
      make_kv_str_key_prop("0", 2.0),
    ];

    assert_eq!(keys_of(&order_own_keys(props)), vec!["0", "<computed>"]);
  }

  // Even when named `0`.
  #[test]
  fn a_getter_named_by_digits_is_not_an_index() {
    let props = vec![make_getter_prop("0"), make_kv_str_key_prop("1", 1.0)];

    assert_eq!(keys_of(&order_own_keys(props)), vec!["1", "<other>"]);
  }

  #[test]
  fn a_big_integer_key_orders_as_the_index_it_spells() {
    let props = vec![
      make_kv_prop("color", 1.0),
      make_bigint_obj_prop(3, vec![]),
      make_kv_str_key_prop("1", 3.0),
    ];

    assert_eq!(keys_of(&order_own_keys(props)), vec!["1", "3", "color"]);
  }

  #[test]
  fn a_shorthand_property_named_by_digits_is_an_index() {
    let props = vec![make_kv_prop("a", 1.0), make_shorthand_prop("0")];

    assert_eq!(keys_of(&order_own_keys(props)), vec!["0", "a"]);
  }
}

mod order_own_map_keys_tests {}
