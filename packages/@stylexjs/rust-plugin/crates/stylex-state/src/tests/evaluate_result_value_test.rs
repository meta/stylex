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

/// Every key shape a property name can take, and the shapes that name none.
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

  /// A bare name reads as itself: `{ color: red }` names the key `color`.
  #[test]
  fn an_identifier_names_itself() {
    assert_eq!(
      key_of(Expr::Ident(create_ident("color"))).as_deref(),
      Some("color")
    );
  }

  /// A quoted key reads as its text, including the spellings no identifier can
  /// take -- a custom property, a media query, an empty string.
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

  /// A bigint key is spelled without the `n` the source carries, which is how
  /// the language spells it.
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

  /// Every value the language spells names a key, because a property key is
  /// `String(key)` -- `null` names the property `null`, and an object names its
  /// default text.
  ///
  /// A table of the kinds a key may be written as answered for a string and a
  /// number only, so a key written as anything else was written nowhere and
  /// found nowhere.
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

  /// A value that is not an expression at all names no key either: the outer
  /// match declines before the key shapes are read.
  #[test]
  fn a_non_expression_value_names_no_key() {
    assert_eq!(EvaluateResultValue::Null.as_string_key(), None);
    assert_eq!(EvaluateResultValue::Vec(vec![]).as_string_key(), None);
  }
}

/// The four accessors that read one variant out of an evaluated value.
mod accessors {
  use indexmap::IndexMap;
  use swc_core::ecma::ast::{Expr, KeyValueProp};

  use stylex_ast::ast::factories::{create_ident, create_key_value_prop_ident};

  use crate::tests::prelude::string_expr;
  use crate::{evaluate_result_value::EvaluateResultValue, theme_ref::ThemeRef};

  /// One value of every variant an accessor is asked about, so a case that must
  /// name the variants it does not read has a list to take them from.
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

  /// An empty list is a list. A `create` call whose namespace holds no rule
  /// evaluates to one, and reading it as absent would lose the namespace.
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

    // The identity is pinned as text rather than re-read off `theme_ref`, so
    // an accessor that answered the wrong reference still fails here.
    assert_eq!(
      value.as_theme_ref().map(ThemeRef::base_id),
      Some("vars.stylex.js//vars")
    );
    assert_eq!(count_answering(|value| value.as_theme_ref().is_some()), 1);
  }
}

/// What `stylex-css` reads off the second argument of a `when` call.
///
/// The marker can arrive as a literal class name, as a proxy standing in for a
/// marker another file defined, or as the compiled object a `defineMarker` call
/// left behind. Each accessor answers for one of those and declines the rest.
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

  /// The object a compiled marker is: the compiled flag that says this compiler
  /// wrote the object, and the marker's own class name.
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
    // The group's own name, spelled out rather than re-derived: comparing
    // against `theme_ref.to_string_value()` would pass on any two wrong halves
    // that agree.
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

  /// The compiled flag can be written before or after the class name, and the
  /// answer is the class name either way: the search for the flag and the search
  /// for the name are separate walks.
  #[test]
  fn the_compiled_flag_may_be_written_after_the_class_name() {
    let mut props = compiled_marker();
    props.reverse();

    assert_eq!(object_value(props).first_css_key(), Some("x1marker"));
  }

  /// An object this compiler did not write carries no compiled flag, so it names
  /// no marker even though it has keys that look like one.
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

  /// The flag must be `true`. A `$$css` written as anything else -- `false`, or
  /// the text `"true"` -- is not the flag.
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

  /// An object carrying the flag and nothing else names no marker: there is no
  /// key left after the flag for the class name to be.
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

  /// An empty object names no marker, which is the same walk finding no flag.
  #[test]
  fn an_empty_object_names_no_marker() {
    assert_eq!(object_value(vec![]).first_css_key(), None);
  }

  /// A key written as a bare name reads the same as a quoted one, because both
  /// are what `Object.keys` walks.
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

  /// Property shapes a compiled marker never carries are stepped over rather
  /// than read: a spread, a shorthand, and a computed key are each skipped, so
  /// the class name is still the first plain key that is not the flag.
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

  /// A value that is not an object names no marker at all.
  #[test]
  fn a_value_that_is_not_an_object_names_no_marker() {
    assert_eq!(EvaluateResultValue::Null.first_css_key(), None);
    assert_eq!(string_value("x1marker").first_css_key(), None);
  }

  /// The class-name prefix is a property of the options and never of a marker,
  /// so an evaluated value answers none whatever it holds.
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

  /// An env object is an evaluated value like any other and answers no marker
  /// question, so a `when` call handed one refuses rather than reading into it.
  #[test]
  fn an_env_object_answers_no_marker_question() {
    let value = EvaluateResultValue::EnvObject(Rc::new(IndexMap::new()));

    assert_eq!(value.as_str_value(), None);
    assert!(!value.is_proxy());
    assert_eq!(value.as_proxy_string(), None);
    assert_eq!(value.first_css_key(), None);
  }
}
