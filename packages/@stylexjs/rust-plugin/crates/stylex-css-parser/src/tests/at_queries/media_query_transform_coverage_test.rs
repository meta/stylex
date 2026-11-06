use super::*;

use swc_core::{
  atoms::Wtf8Atom,
  common::DUMMY_SP,
  ecma::ast::{Expr, KeyValueProp, Lit, Number, ObjectLit, Prop, PropName, PropOrSpread, Str},
};

fn str_kv(key: &str, value: &str) -> KeyValueProp {
  KeyValueProp {
    key: PropName::Str(Str {
      span: DUMMY_SP,
      value: Wtf8Atom::from(key),
      raw: None,
    }),
    value: Box::new(Expr::Lit(Lit::Str(Str {
      span: DUMMY_SP,
      value: Wtf8Atom::from(value),
      raw: None,
    }))),
  }
}

fn num_kv(key: f64, value: &str) -> KeyValueProp {
  KeyValueProp {
    key: PropName::Num(Number {
      span: DUMMY_SP,
      value: key,
      raw: None,
    }),
    value: Box::new(Expr::Lit(Lit::Str(Str {
      span: DUMMY_SP,
      value: Wtf8Atom::from(value),
      raw: None,
    }))),
  }
}

#[cfg(test)]
mod key_value_str_coverage {}

#[cfg(test)]
mod dfs_coverage {}

#[cfg(test)]
mod transform_media_coverage {}

#[cfg(test)]
mod ident_key_integration {
  use super::*;

  /// Two numeric keys stay two properties, as in a JavaScript object, rather than merging.
  #[test]
  fn two_unrenderable_keys_stay_two_properties() {
    let inner_obj = ObjectLit {
      span: DUMMY_SP,
      props: vec![
        PropOrSpread::Prop(Box::new(Prop::KeyValue(str_kv("default", "1 / 2")))),
        PropOrSpread::Prop(Box::new(Prop::KeyValue(num_kv(0.0, "zero")))),
        PropOrSpread::Prop(Box::new(Prop::KeyValue(num_kv(1.0, "one")))),
        // A media key, so the rewrite runs at all rather than returning early.
        PropOrSpread::Prop(Box::new(Prop::KeyValue(str_kv(
          "@media (min-width: 100px)",
          "1 / 3",
        )))),
      ],
    };

    let outer_prop = KeyValueProp {
      key: PropName::Str(Str {
        span: DUMMY_SP,
        value: Wtf8Atom::from("gridColumn"),
        raw: None,
      }),
      value: Box::new(Expr::Object(inner_obj)),
    };

    let result = last_media_query_wins_transform(&[outer_prop]);

    match &*result[0].value {
      Expr::Object(inner) => assert_eq!(inner.props.len(), 4),
      other => panic!("Expected Object value, got {other:?}"),
    }
  }
}
