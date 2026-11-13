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

mod string_key {}

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
  fn a_list_answers_its_elements_and_nothing_else_does() {
    let elements = vec![EvaluateResultValue::Null];

    assert_eq!(
      EvaluateResultValue::Vec(elements.clone()).as_vec(),
      Some(&elements)
    );
    assert_eq!(count_answering(|value| value.as_vec().is_some()), 1);
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
  use stylex_ast::ast::factories::{create_ident, create_object_lit, create_str_key_value_prop};
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
