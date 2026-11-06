use crate::evaluate_result_value::EvaluateResultValue;

#[test]
fn serializes_null_as_json_null() {
  let json = match serde_json::to_string(&EvaluateResultValue::Null) {
    Ok(json) => json,
    Err(error) => panic!("failed to serialize null evaluate result: {error}"),
  };

  assert_eq!(json, "null");
}

#[test]
fn deserializes_json_null_as_null() {
  let value = match serde_json::from_str::<EvaluateResultValue>("null") {
    Ok(value) => value,
    Err(error) => panic!("failed to deserialize null evaluate result: {error}"),
  };

  assert_eq!(value, EvaluateResultValue::Null);
}

mod string_key {
  use swc_core::{
    common::DUMMY_SP,
    ecma::ast::{BigInt, Expr, Lit, Null, ObjectLit, Str},
  };

  use stylex_ast::ast::factories::create_ident;

  use crate::evaluate_result_value::EvaluateResultValue;

  fn key_of(expr: Expr) -> Option<String> {
    EvaluateResultValue::Expr(expr).as_string_key()
  }

  #[test]
  fn an_identifier_names_itself() {
    assert_eq!(
      key_of(Expr::Ident(create_ident("color"))).as_deref(),
      Some("color")
    );
  }

  #[test]
  fn a_string_names_its_text() {
    for text in ["color", "--custom-prop", "@media (min-width: 320px)", ""] {
      assert_eq!(
        key_of(Expr::Lit(Lit::Str(Str {
          span: DUMMY_SP,
          value: text.into(),
          raw: None,
        })))
        .as_deref(),
        Some(text)
      );
    }
  }

  #[test]
  fn a_bigint_names_its_digits() {
    assert_eq!(
      key_of(Expr::Lit(Lit::BigInt(BigInt {
        span: DUMMY_SP,
        value: Box::new(123_456_789_012_345_678_901_234_567_890u128.into()),
        raw: None,
      })))
      .as_deref(),
      Some("123456789012345678901234567890")
    );
  }

  #[test]
  fn every_value_the_language_spells_names_a_key() {
    assert_eq!(
      key_of(Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))).as_deref(),
      Some("null")
    );
    assert_eq!(
      key_of(Expr::Object(ObjectLit {
        span: DUMMY_SP,
        props: vec![],
      }))
      .as_deref(),
      Some("[object Object]")
    );
  }

  #[test]
  fn a_non_expression_value_names_no_key() {
    assert_eq!(EvaluateResultValue::Null.as_string_key(), None);
    assert_eq!(EvaluateResultValue::Vec(vec![]).as_string_key(), None);
  }
}

mod accessors {
  use indexmap::IndexMap;
  use swc_core::ecma::ast::{Expr, KeyValueProp};

  use stylex_ast::ast::factories::{create_ident, create_key_value_prop_ident};

  use crate::tests::prelude::string_expr;
  use crate::{evaluate_result_value::EvaluateResultValue, theme_ref::ThemeRef};

  fn every_readable_variant() -> Vec<EvaluateResultValue> {
    let mut map: IndexMap<Expr, Vec<KeyValueProp>> = IndexMap::new();

    map.insert(
      Expr::Ident(create_ident("base")),
      vec![create_key_value_prop_ident("color", string_expr("red"))],
    );

    vec![
      EvaluateResultValue::Null,
      EvaluateResultValue::Expr(string_expr("red")),
      EvaluateResultValue::Vec(vec![EvaluateResultValue::Null]),
      EvaluateResultValue::Map(map),
      EvaluateResultValue::ThemeRef(ThemeRef::new("vars.stylex.js", "vars", "x")),
    ]
  }

  fn count_answering(accessor: impl Fn(&EvaluateResultValue) -> bool) -> usize {
    every_readable_variant()
      .iter()
      .filter(|value| accessor(value))
      .count()
  }

  #[test]
  fn an_expression_answers_itself_and_nothing_else_does() {
    let expr = string_expr("red");

    assert_eq!(
      EvaluateResultValue::Expr(expr.clone()).as_expr(),
      Some(&expr)
    );
    assert_eq!(count_answering(|value| value.as_expr().is_some()), 1);
  }

  #[test]
  fn a_list_answers_its_elements_and_nothing_else_does() {
    let elements = vec![EvaluateResultValue::Null];

    assert_eq!(
      EvaluateResultValue::Vec(elements.clone()).as_vec(),
      Some(&elements)
    );
    assert_eq!(count_answering(|value| value.as_vec().is_some()), 1);
  }

  #[test]
  fn an_empty_list_is_still_a_list() {
    assert_eq!(EvaluateResultValue::Vec(vec![]).as_vec(), Some(&Vec::new()));
  }

  #[test]
  fn a_map_answers_its_entries_and_nothing_else_does() {
    let value = every_readable_variant().remove(3);

    assert_eq!(value.as_map().map(IndexMap::len), Some(1));
    assert_eq!(count_answering(|value| value.as_map().is_some()), 1);
  }

  #[test]
  fn a_theme_reference_answers_itself_and_nothing_else_does() {
    let value = EvaluateResultValue::ThemeRef(ThemeRef::new("vars.stylex.js", "vars", "x"));

    assert_eq!(
      value.as_theme_ref().map(ThemeRef::base_id),
      Some("vars.stylex.js//vars")
    );
    assert_eq!(count_answering(|value| value.as_theme_ref().is_some()), 1);
  }
}

mod when_marker {
  use std::rc::Rc;

  use indexmap::IndexMap;
  use swc_core::{
    common::DUMMY_SP,
    ecma::ast::{
      ComputedPropName, Expr, KeyValueProp, Lit, Prop, PropName, PropOrSpread, SpreadElement, Str,
    },
  };

  use stylex_ast::ast::convertors::create_bool_expr;
  use stylex_ast::ast::factories::{
    create_ident, create_ident_key_value_prop, create_object_lit, create_str_key_value_prop,
  };
  use stylex_types::traits::WhenMarkerValue;

  use crate::{evaluate_result_value::EvaluateResultValue, theme_ref::ThemeRef};

  fn string_value(text: &str) -> EvaluateResultValue {
    EvaluateResultValue::Expr(Expr::Lit(Lit::Str(Str {
      span: DUMMY_SP,
      value: text.into(),
      raw: None,
    })))
  }

  fn object_value(props: Vec<PropOrSpread>) -> EvaluateResultValue {
    EvaluateResultValue::Expr(Expr::Object(create_object_lit(props)))
  }

  fn compiled_marker() -> Vec<PropOrSpread> {
    vec![
      create_str_key_value_prop("$$css", create_bool_expr(true)),
      create_str_key_value_prop("x1marker", create_bool_expr(true)),
    ]
  }

  #[test]
  fn a_literal_marker_is_read_as_written() {
    assert_eq!(string_value("x1marker").as_str_value(), Some("x1marker"));
  }

  #[test]
  fn a_value_that_is_not_a_string_carries_no_literal_marker() {
    assert_eq!(EvaluateResultValue::Null.as_str_value(), None);
    assert_eq!(object_value(compiled_marker()).as_str_value(), None);
  }

  #[test]
  fn a_theme_reference_is_a_proxy_and_resolves_through_its_own_name() {
    let theme_ref = ThemeRef::new("markers.stylex.js", "marker", "x");
    let value = EvaluateResultValue::ThemeRef(theme_ref.clone());

    assert!(value.is_proxy());

    assert_eq!(value.as_proxy_string(), Some("x3hqbbp".to_string()));
    assert_eq!(value.as_proxy_string(), Some(theme_ref.to_string_value()));
  }

  #[test]
  fn nothing_else_is_a_proxy() {
    for value in [
      EvaluateResultValue::Null,
      string_value("x1marker"),
      object_value(compiled_marker()),
    ] {
      assert!(!value.is_proxy());
      assert_eq!(value.as_proxy_string(), None);
    }
  }

  #[test]
  fn a_compiled_marker_object_answers_its_one_class_name() {
    assert_eq!(
      object_value(compiled_marker()).first_css_key(),
      Some("x1marker")
    );
  }

  #[test]
  fn the_compiled_flag_may_be_written_after_the_class_name() {
    let mut props = compiled_marker();
    props.reverse();

    assert_eq!(object_value(props).first_css_key(), Some("x1marker"));
  }

  #[test]
  fn an_object_without_the_compiled_flag_names_no_marker() {
    assert_eq!(
      object_value(vec![create_str_key_value_prop(
        "x1marker",
        create_bool_expr(true)
      )])
      .first_css_key(),
      None
    );
  }

  #[test]
  fn a_compiled_flag_that_is_not_true_names_no_marker() {
    for flag in [
      create_bool_expr(false),
      Expr::Lit(Lit::Str(Str {
        span: DUMMY_SP,
        value: "true".into(),
        raw: None,
      })),
    ] {
      assert_eq!(
        object_value(vec![
          create_str_key_value_prop("$$css", flag),
          create_str_key_value_prop("x1marker", create_bool_expr(true)),
        ])
        .first_css_key(),
        None
      );
    }
  }

  #[test]
  fn an_object_holding_only_the_compiled_flag_names_no_marker() {
    assert_eq!(
      object_value(vec![create_str_key_value_prop(
        "$$css",
        create_bool_expr(true)
      )])
      .first_css_key(),
      None
    );
  }

  #[test]
  fn an_empty_object_names_no_marker() {
    assert_eq!(object_value(vec![]).first_css_key(), None);
  }

  #[test]
  fn a_marker_key_written_as_a_bare_name_reads_the_same() {
    assert_eq!(
      object_value(vec![
        create_ident_key_value_prop("$$css", create_bool_expr(true)),
        create_ident_key_value_prop("marker", create_bool_expr(true)),
      ])
      .first_css_key(),
      Some("marker")
    );
  }

  #[test]
  fn shapes_a_compiled_object_never_carries_are_stepped_over() {
    let props = vec![
      PropOrSpread::Spread(SpreadElement {
        dot3_token: DUMMY_SP,
        expr: Box::new(Expr::Ident(create_ident("base"))),
      }),
      create_str_key_value_prop("$$css", create_bool_expr(true)),
      PropOrSpread::Prop(Box::new(Prop::Shorthand(create_ident("shorthand")))),
      PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Computed(ComputedPropName {
          span: DUMMY_SP,
          expr: Box::new(Expr::Ident(create_ident("computed"))),
        }),
        value: Box::new(create_bool_expr(true)),
      }))),
      create_str_key_value_prop("x1marker", create_bool_expr(true)),
    ];

    assert_eq!(object_value(props).first_css_key(), Some("x1marker"));
  }

  #[test]
  fn a_value_that_is_not_an_object_names_no_marker() {
    assert_eq!(EvaluateResultValue::Null.first_css_key(), None);
    assert_eq!(string_value("x1marker").first_css_key(), None);
  }

  #[test]
  fn an_evaluated_value_never_carries_a_class_name_prefix() {
    for value in [
      EvaluateResultValue::Null,
      string_value("x1marker"),
      object_value(compiled_marker()),
      EvaluateResultValue::ThemeRef(ThemeRef::new("markers.stylex.js", "marker", "x")),
    ] {
      assert_eq!(value.class_name_prefix(), None);
    }
  }

  #[test]
  fn an_env_object_answers_no_marker_question() {
    let value = EvaluateResultValue::EnvObject(Rc::new(IndexMap::new()));

    assert_eq!(value.as_str_value(), None);
    assert!(!value.is_proxy());
    assert_eq!(value.as_proxy_string(), None);
    assert_eq!(value.first_css_key(), None);
  }
}
