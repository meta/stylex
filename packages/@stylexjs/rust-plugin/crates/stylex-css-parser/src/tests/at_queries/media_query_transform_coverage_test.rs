use super::*;

use swc_core::{
  atoms::Wtf8Atom,
  common::DUMMY_SP,
  ecma::ast::{
    ArrayLit, Expr, Ident, KeyValueProp, Lit, Number, ObjectLit, Prop, PropName, PropOrSpread, Str,
  },
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

fn ident_kv(key: &str, value: &str) -> KeyValueProp {
  KeyValueProp {
    key: PropName::Ident(key.into()),
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
mod key_value_str_coverage {
  use super::*;

  #[test]
  fn ident_key_returns_sym_string() {
    let kv = ident_kv("gridColumn", "1 / 2");
    assert_eq!(key_value_str(&kv), Some("gridColumn"));
  }

  /// A `PropName::Num` key reads as `None`, which the caller maps to a positional key.
  #[test]
  fn numeric_key_returns_no_name() {
    let kv = num_kv(42.0, "value");
    assert_eq!(key_value_str(&kv), None);
  }

  #[test]
  fn str_key_returns_value_string() {
    let kv = str_kv("color", "red");
    assert_eq!(key_value_str(&kv), Some("color"));
  }
}

#[cfg(test)]
mod dfs_coverage {
  use super::*;

  #[test]
  fn array_valued_prop_passes_through_unchanged() {
    let array_expr = Expr::Array(ArrayLit {
      span: DUMMY_SP,
      elems: vec![],
    });

    let prop = KeyValueProp {
      key: PropName::Str(Str {
        span: DUMMY_SP,
        value: Wtf8Atom::from("firstThatWorks"),
        raw: None,
      }),
      value: Box::new(array_expr),
    };

    let result = last_media_query_wins_transform(&[prop]);

    assert_eq!(result.len(), 1);
    if let PropName::Str(s) = &result[0].key {
      assert_eq!(s.value.as_str().unwrap(), "firstThatWorks");
    } else {
      panic!("Expected Str key");
    }
    assert!(matches!(&*result[0].value, Expr::Array(_)));
  }

  /// Non-KeyValue props are preserved so StyleX validation can report the non-static value.
  #[test]
  fn object_with_shorthand_prop_skips_non_key_value() {
    let shorthand_prop = Prop::Shorthand(Ident::from("foo"));

    let obj_lit = ObjectLit {
      span: DUMMY_SP,
      props: vec![PropOrSpread::Prop(Box::new(shorthand_prop))],
    };

    let outer_prop = KeyValueProp {
      key: PropName::Str(Str {
        span: DUMMY_SP,
        value: Wtf8Atom::from("outer"),
        raw: None,
      }),
      value: Box::new(Expr::Object(obj_lit)),
    };

    let result = last_media_query_wins_transform(&[outer_prop]);

    assert_eq!(result.len(), 1);
    if let Expr::Object(inner_obj) = &*result[0].value {
      assert_eq!(inner_obj.props.len(), 1);
      assert!(matches!(
        inner_obj.props[0],
        PropOrSpread::Prop(ref prop) if matches!(prop.as_ref(), Prop::Shorthand(_))
      ));
    } else {
      panic!("Expected Object value");
    }
  }

  #[test]
  fn object_with_spread_prop_is_skipped() {
    use swc_core::ecma::ast::SpreadElement;

    let spread = SpreadElement {
      dot3_token: DUMMY_SP,
      expr: Box::new(Expr::Lit(Lit::Str(Str {
        span: DUMMY_SP,
        value: Wtf8Atom::from("irrelevant"),
        raw: None,
      }))),
    };

    let obj_lit = ObjectLit {
      span: DUMMY_SP,
      props: vec![PropOrSpread::Spread(spread)],
    };

    let outer_prop = KeyValueProp {
      key: PropName::Str(Str {
        span: DUMMY_SP,
        value: Wtf8Atom::from("outerSpread"),
        raw: None,
      }),
      value: Box::new(Expr::Object(obj_lit)),
    };

    let result = last_media_query_wins_transform(&[outer_prop]);

    assert_eq!(result.len(), 1);
    if let Expr::Object(inner_obj) = &*result[0].value {
      assert_eq!(inner_obj.props.len(), 1);
      assert!(matches!(inner_obj.props[0], PropOrSpread::Spread(_)));
    } else {
      panic!("Expected Object value");
    }
  }
}

#[cfg(test)]
mod transform_media_coverage {
  use super::*;

  #[test]
  fn multiple_media_queries_fill_accumulated_negations() {
    let props = vec![
      str_kv("default", "1 / 2"),
      str_kv("@media (color)", "colorful"),
      str_kv("@media (monochrome)", "grayscale"),
    ];

    let result = transform_media_queries_in_result(props);

    assert!(!result.is_empty());
    let has_negation = result.iter().any(|kv| {
      if let PropName::Str(s) = &kv.key {
        let k = s.value.as_str().unwrap_or("");
        k.contains("not")
      } else {
        false
      }
    });
    assert!(
      has_negation,
      "Expected at least one media query with a 'not' negation"
    );
  }

  #[test]
  fn three_media_queries_produce_multiple_accumulated_negation_entries() {
    let props = vec![
      str_kv("@media (color)", "colorful"),
      str_kv("@media (monochrome)", "grayscale"),
      str_kv("@media (hover: hover)", "hoverable"),
    ];

    let result = transform_media_queries_in_result(props);

    assert_eq!(result.len(), 3);
  }

  /// An unparseable media key is rejected rather than emitted verbatim; nothing downstream
  /// validates it.
  #[test]
  #[should_panic(expected = "Invalid media query")]
  fn media_key_that_fails_to_parse_is_rejected() {
    let props = vec![
      str_kv("@media (color)", "red"),
      str_kv("@media !!!invalid!!!css", "blue"),
    ];

    transform_media_queries_in_result(props);
  }

  /// A nested invalid media key is rejected end to end, not only through the direct call.
  #[test]
  #[should_panic(expected = "Invalid media query")]
  fn invalid_media_key_via_transform_is_rejected() {
    let inner_obj = ObjectLit {
      span: DUMMY_SP,
      props: vec![
        PropOrSpread::Prop(Box::new(Prop::KeyValue(str_kv(
          "@media (max-width: 768px)",
          "red",
        )))),
        PropOrSpread::Prop(Box::new(Prop::KeyValue(str_kv(
          "@media !!!invalid",
          "blue",
        )))),
      ],
    };

    let outer_prop = KeyValueProp {
      key: PropName::Str(Str {
        span: DUMMY_SP,
        value: Wtf8Atom::from("color"),
        raw: None,
      }),
      value: Box::new(Expr::Object(inner_obj)),
    };

    last_media_query_wins_transform(&[outer_prop]);
  }
}

#[cfg(test)]
mod ident_key_integration {
  use super::*;

  #[test]
  fn ident_keyed_style_prop_passes_through() {
    let inner_obj = ObjectLit {
      span: DUMMY_SP,
      props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(ident_kv(
        "default", "1 / 2",
      ))))],
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

    assert_eq!(result.len(), 1);
    if let Expr::Object(inner) = &*result[0].value {
      assert_eq!(inner.props.len(), 1);
    } else {
      panic!("Expected Object value");
    }
  }

  /// A Num-keyed prop at depth 1 is treated as non-media and passes through unchanged.
  #[test]
  fn numeric_keyed_prop_in_nested_object_does_not_match_media_query() {
    let inner_obj = ObjectLit {
      span: DUMMY_SP,
      props: vec![
        PropOrSpread::Prop(Box::new(Prop::KeyValue(str_kv("default", "1 / 2")))),
        PropOrSpread::Prop(Box::new(Prop::KeyValue(num_kv(0.0, "zero")))),
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

    assert_eq!(result.len(), 1);
    if let Expr::Object(inner) = &*result[0].value {
      assert_eq!(inner.props.len(), 2);
    } else {
      panic!("Expected Object value");
    }
  }

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
