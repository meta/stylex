#[cfg(test)]
mod class_methods {
  use indexmap::IndexMap;
  use swc_core::ecma::ast::Expr;

  use crate::shared::transformers::stylex_types::{FN_MAP, get_types_fn};
  use stylex_ast::ast::factories::{
    create_key_value_prop, create_object_expression, create_string_key_value_prop,
  };
  use stylex_enums::value_with_default::ValueWithDefault;
  use stylex_state::functions::FunctionType;

  fn assert_type_factory(syntax: &str, value: ValueWithDefault, expected_value: &str) {
    let angle = FN_MAP.get(syntax).unwrap();

    let result = angle(value);

    assert_eq!(result, type_factory(syntax, expected_value));
  }

  fn type_factory(syntax: &str, value: &str) -> Expr {
    create_object_expression(vec![
      create_string_key_value_prop("syntax", format!("<{}>", syntax).as_str()),
      create_string_key_value_prop("value", value),
    ])
  }

  #[test]
  fn angle() {
    assert_type_factory(
      "angle",
      ValueWithDefault::String("45deg".to_string()),
      "45deg",
    );
  }

  #[test]
  fn color() {
    assert_type_factory("color", ValueWithDefault::String("red".to_string()), "red");
  }

  #[test]
  fn image() {
    assert_type_factory(
      "image",
      ValueWithDefault::String("url(#image)".to_string()),
      "url(#image)",
    );
  }

  #[test]
  fn integer() {
    assert_type_factory("integer", ValueWithDefault::Number(1.0), "1");
  }

  #[test]
  fn length() {
    let value = "1px";

    assert_type_factory("length", ValueWithDefault::String(value.to_string()), value);

    assert_type_factory("length", ValueWithDefault::Number(1.0), value);
  }

  #[test]
  fn percentage() {
    let value = "50%";

    assert_type_factory(
      "percentage",
      ValueWithDefault::String(value.to_string()),
      value,
    );

    assert_type_factory("percentage", ValueWithDefault::Number(0.5), value);
  }

  #[test]
  fn number() {
    assert_type_factory("number", ValueWithDefault::Number(1.0), "1");
  }

  #[test]
  fn resolution() {
    let value = "96dpi";

    assert_type_factory(
      "resolution",
      ValueWithDefault::String(value.to_string()),
      value,
    );
  }

  #[test]
  fn time() {
    let value = "1s";

    assert_type_factory("time", ValueWithDefault::String(value.to_string()), value);
  }

  #[test]
  fn transform_function() {
    let value = "translateX(10px)";

    assert_type_factory(
      "transformFunction",
      ValueWithDefault::String(value.to_string()),
      value,
    );
  }

  #[test]
  fn transform_list() {
    let value = "translateX(10px)";

    assert_type_factory(
      "transformList",
      ValueWithDefault::String(value.to_string()),
      value,
    );
  }

  #[test]
  fn url() {
    let value = "url(#image)";

    assert_type_factory("url", ValueWithDefault::String(value.to_string()), value);
  }

  #[test]
  fn length_percentage() {
    assert_type_factory("lengthPercentage", ValueWithDefault::Number(0.5), "50%");

    assert_type_factory(
      "lengthPercentage",
      ValueWithDefault::String("1px".to_string()),
      "1px",
    );
  }

  /// Zero is the one number a length and a percentage write without a unit,
  /// because `0px` and `0%` both mean `0`.
  mod a_zero_value {
    use super::*;

    #[test]
    fn keeps_no_unit_on_a_length() {
      assert_type_factory("length", ValueWithDefault::Number(0.0), "0");
    }

    #[test]
    fn keeps_no_unit_on_a_percentage() {
      assert_type_factory("percentage", ValueWithDefault::Number(0.0), "0");
    }

    #[test]
    fn keeps_no_unit_on_a_length_percentage() {
      assert_type_factory("lengthPercentage", ValueWithDefault::Number(0.0), "0");
    }
  }

  /// A map carries a value per at-rule. Every number below it is converted the
  /// same way a value written on its own is, however deep it sits.
  mod a_map_value {
    use super::*;

    fn map(entries: &[(&str, ValueWithDefault)]) -> ValueWithDefault {
      ValueWithDefault::Map(
        entries
          .iter()
          .map(|(key, value)| ((*key).to_string(), value.clone()))
          .collect::<IndexMap<_, _>>(),
      )
    }

    /// The four value shapes in one call: a number, a string that reads as a
    /// number, a string that does not, and a map below the map.
    #[test]
    fn converts_every_number_it_holds_and_leaves_the_rest() {
      let result = FN_MAP.get("length").unwrap()(map(&[
        ("default", ValueWithDefault::Number(1.0)),
        ("@media print", ValueWithDefault::String("2".to_string())),
        (
          "@media screen",
          ValueWithDefault::String("auto".to_string()),
        ),
        (
          "@supports (display: grid)",
          map(&[("default", ValueWithDefault::Number(3.0))]),
        ),
      ]));

      assert_eq!(
        result,
        create_object_expression(vec![
          create_string_key_value_prop("syntax", "<length>"),
          create_key_value_prop(
            "value",
            create_object_expression(vec![
              create_string_key_value_prop("default", "1px"),
              create_string_key_value_prop("@media print", "2px"),
              create_string_key_value_prop("@media screen", "auto"),
              create_key_value_prop(
                "@supports (display: grid)",
                create_object_expression(vec![create_string_key_value_prop("default", "3px")])
              ),
            ])
          ),
        ])
      );
    }

    /// A map that holds nothing is answered by a value object that holds
    /// nothing, rather than by a refusal.
    #[test]
    fn is_answered_by_an_empty_value_object_when_it_holds_nothing() {
      let result = FN_MAP.get("number").unwrap()(map(&[]));

      assert_eq!(
        result,
        create_object_expression(vec![
          create_string_key_value_prop("syntax", "<number>"),
          create_key_value_prop("value", create_object_expression(vec![])),
        ])
      );
    }
  }

  mod the_type_factory {
    use super::*;

    fn factory_of(name: &str) -> Expr {
      let FunctionType::StylexFnsFactory(factory) = get_types_fn().fn_ptr else {
        panic!("the types function is a factory of type functions")
      };

      factory(name.to_string())(ValueWithDefault::String("1px".to_string()))
    }

    #[test]
    fn answers_the_function_the_name_stands_for() {
      assert_eq!(factory_of("length"), type_factory("length", "1px"));
    }

    #[test]
    #[should_panic(expected = r#"Function "notAType" not found"#)]
    fn refuses_a_name_no_type_is_written_under() {
      let _ = factory_of("notAType");
    }
  }
}
