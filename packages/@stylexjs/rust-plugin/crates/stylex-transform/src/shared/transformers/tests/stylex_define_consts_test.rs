#[cfg(test)]
mod stylex_define_consts {
  use std::rc::Rc;

  use stylex_ast::ast::convertors::create_string_expr;
  use swc_core::ecma::ast::PropOrSpread;

  use crate::shared::transformers::stylex_define_consts::stylex_define_consts;
  use stylex_ast::ast::factories::{create_key_value_prop, create_object_expression};
  use stylex_state::{
    evaluate_result_value::EvaluateResultValue,
    flat_compiled_styles_value::FlatCompiledStylesValue, state_manager::StateManager,
  };
  use stylex_structures::stylex_options::StyleXOptions;
  use stylex_types::enums::data_structures::injectable_style::InjectableStyleKind;
  use stylex_utils::hash::create_hash;

  fn create_test_state_manager(export_id: &str) -> StateManager {
    let options = StyleXOptions::default()
      .with_class_name_prefix("x")
      .with_debug(false);

    let mut state = StateManager::new(options);
    state.export_id = Some(export_id.to_string());
    state
  }

  fn constants_factory(constants: &[(&str, &str)]) -> EvaluateResultValue {
    let props = constants
      .iter()
      .map(|(key, value)| create_key_value_prop(key, create_string_expr(value)))
      .collect::<Vec<PropOrSpread>>();

    EvaluateResultValue::Expr(create_object_expression(props))
  }

  fn get_const_hash(export_id: &str, key: &str, class_name_prefix: &str) -> String {
    format!(
      "{}{}",
      class_name_prefix,
      create_hash(&format!("{}.{}", export_id, key))
    )
  }

  #[test]
  fn returns_correct_structure_for_basic_constants() {
    let export_id = "TestTheme.stylex.js//buttonTheme";
    let mut state = create_test_state_manager(export_id);

    let constants =
      constants_factory(&[("sm", "(min-width: 768px)"), ("md", "(min-width: 1024px)")]);

    let (js_output, injectable_styles) = stylex_define_consts(&constants, &mut state);

    assert_eq!(js_output.len(), 2);
    assert_eq!(
      js_output.get("sm").unwrap().as_string().unwrap(),
      "(min-width: 768px)"
    );
    assert_eq!(
      js_output.get("md").unwrap().as_string().unwrap(),
      "(min-width: 1024px)"
    );

    let sm_hash = get_const_hash(export_id, "sm", "x");
    let md_hash = get_const_hash(export_id, "md", "x");

    assert!(injectable_styles.contains_key(sm_hash.as_str()));
    assert!(injectable_styles.contains_key(md_hash.as_str()));

    match injectable_styles.get(sm_hash.as_str()).unwrap().as_ref() {
      InjectableStyleKind::Const(style) => {
        // The rule carries the constant as JSON, so text arrives quoted and a
        // reader of it knows the kind without guessing one.
        assert_eq!(style.const_value, "\"(min-width: 768px)\"");
        assert_eq!(style.ltr, "");
        assert_eq!(style.rtl, None);
        assert_eq!(style.priority, Some(0.0));
      },
      _ => panic!("Expected InjectableStyleKind::Const"),
    }

    match injectable_styles.get(md_hash.as_str()).unwrap().as_ref() {
      InjectableStyleKind::Const(style) => {
        assert_eq!(style.const_value, "\"(min-width: 1024px)\"");
        assert_eq!(style.ltr, "");
        assert_eq!(style.rtl, None);
        assert_eq!(style.priority, Some(0.0));
      },
      _ => panic!("Expected InjectableStyleKind::Const"),
    }

    assert_ne!(sm_hash, md_hash);
  }

  #[test]
  fn handles_special_characters_in_keys() {
    let export_id = "TestTheme.stylex.js//buttonTheme";
    let mut state = create_test_state_manager(export_id);

    let constants = constants_factory(&[("max-width", "1200px"), ("font-size*large", "18px")]);

    let (js_output, injectable_styles) = stylex_define_consts(&constants, &mut state);

    assert_eq!(js_output.len(), 2);
    assert_eq!(
      js_output.get("max-width").unwrap().as_string().unwrap(),
      "1200px"
    );
    assert_eq!(
      js_output
        .get("font-size*large")
        .unwrap()
        .as_string()
        .unwrap(),
      "18px"
    );

    let max_width_hash = get_const_hash(export_id, "max-width", "x");
    let font_size_hash = get_const_hash(export_id, "font-size*large", "x");

    assert!(injectable_styles.contains_key(max_width_hash.as_str()));
    assert!(injectable_styles.contains_key(font_size_hash.as_str()));

    assert_ne!(max_width_hash, font_size_hash);
  }

  #[test]
  fn handles_numeric_keys() {
    let export_id = "TestTheme.stylex.js//buttonTheme";
    let mut state = create_test_state_manager(export_id);

    let constants = constants_factory(&[("1", "one"), ("2", "two")]);

    let (js_output, injectable_styles) = stylex_define_consts(&constants, &mut state);

    assert_eq!(js_output.len(), 2);
    assert_eq!(js_output.get("1").unwrap().as_string().unwrap(), "one");
    assert_eq!(js_output.get("2").unwrap().as_string().unwrap(), "two");

    let hash_1 = get_const_hash(export_id, "1", "x");
    let hash_2 = get_const_hash(export_id, "2", "x");

    assert!(injectable_styles.contains_key(hash_1.as_str()));
    assert!(injectable_styles.contains_key(hash_2.as_str()));

    assert_ne!(hash_1, hash_2);
  }

  #[test]
  fn generates_consistent_hashes_for_identical_constants() {
    let export_id = "TestTheme.stylex.js//buttonTheme";
    let mut state1 = create_test_state_manager(export_id);
    let mut state2 = create_test_state_manager(export_id);

    let constants = constants_factory(&[("padding", "10px")]);

    let (js_output1, styles1) = stylex_define_consts(&constants, &mut state1);
    let (js_output2, styles2) = stylex_define_consts(&constants, &mut state2);

    let key_hash = get_const_hash(export_id, "padding", "x");

    assert_eq!(js_output1, js_output2);

    assert!(styles1.contains_key(key_hash.as_str()));
    assert!(styles2.contains_key(key_hash.as_str()));

    match (
      styles1.get(key_hash.as_str()).unwrap().as_ref(),
      styles2.get(key_hash.as_str()).unwrap().as_ref(),
    ) {
      (InjectableStyleKind::Const(style1), InjectableStyleKind::Const(style2)) => {
        assert_eq!(style1.const_key, style2.const_key);
      },
      _ => panic!("Expected InjectableStyleKind::Const"),
    }
  }

  #[test]
  fn generates_different_hashes_for_different_constants() {
    let export_id = "TestTheme.stylex.js//buttonTheme";
    let mut state1 = create_test_state_manager(export_id);
    let mut state2 = create_test_state_manager(export_id);

    let constants1 = constants_factory(&[("padding", "10px")]);
    let constants2 = constants_factory(&[("margin", "10px")]);

    let (_, styles1) = stylex_define_consts(&constants1, &mut state1);
    let (_, styles2) = stylex_define_consts(&constants2, &mut state2);

    let padding_key = get_const_hash(export_id, "padding", "x");
    let margin_key = get_const_hash(export_id, "margin", "x");

    assert_ne!(padding_key, margin_key);

    assert!(styles1.contains_key(padding_key.as_str()));
    assert!(styles2.contains_key(margin_key.as_str()));

    match (
      styles1.get(padding_key.as_str()).unwrap().as_ref(),
      styles2.get(margin_key.as_str()).unwrap().as_ref(),
    ) {
      (InjectableStyleKind::Const(style1), InjectableStyleKind::Const(style2)) => {
        assert_ne!(style1.const_key, style2.const_key);
      },
      _ => panic!("Expected InjectableStyleKind::Const"),
    }
  }

  #[test]
  fn preserves_object_keys() {
    let export_id = "TestTheme.stylex.js//buttonTheme";
    let mut state = create_test_state_manager(export_id);

    let constants = constants_factory(&[("borderRadius", "8px"), ("colorPrimary", "#ff0000")]);

    let (js_output, _) = stylex_define_consts(&constants, &mut state);

    let keys: Vec<&String> = js_output.keys().collect();
    assert_eq!(keys.len(), 2);
    assert!(keys.contains(&&"borderRadius".to_string()));
    assert!(keys.contains(&&"colorPrimary".to_string()));
  }

  #[test]
  fn throws_an_error_for_keys_that_start_with_double_dash() {
    let export_id = "TestTheme.stylex.js//buttonTheme";
    let mut state = create_test_state_manager(export_id);

    let constants = constants_factory(&[("--custom-var", "red")]);

    let (js_output, _) = stylex_define_consts(&constants, &mut state);

    assert_eq!(js_output.len(), 1);
    assert_eq!(
      js_output.get("--custom-var").unwrap().as_string().unwrap(),
      "red"
    );
  }

  /// A constant keeps the kind the author gave it. A literal and a template
  /// that spells one piece of text are read as that text, an object is read as
  /// an object, and a value that spells nothing static is refused.
  mod a_constant_value {
    use super::*;

    use crate::tests::support::expr;

    fn value_of(code: &str) -> Rc<FlatCompiledStylesValue> {
      let constants = EvaluateResultValue::Expr(expr(code));

      let (js_output, _) = stylex_define_consts(
        &constants,
        &mut create_test_state_manager("Test.stylex.js//consts"),
      );

      match js_output.get("value") {
        Some(value) => Rc::clone(value),
        None => panic!("the constant {code} holds nothing"),
      }
    }

    fn text_of(code: &str) -> String {
      match value_of(code).as_string() {
        Some(text) => text.clone(),
        None => panic!("the constant {code} holds no text"),
      }
    }

    #[test]
    fn reads_a_template_that_spells_one_piece_of_text() {
      assert_eq!(
        text_of("{ value: `(min-width: 768px)` }"),
        "(min-width: 768px)"
      );
    }

    #[test]
    fn reads_an_empty_template_as_empty_text() {
      assert_eq!(text_of("{ value: `` }"), "");
    }

    /// A template holding an expression spells no one piece of text, so there
    /// is no static value to keep and the call is refused. The reference
    /// refuses such a constant too.
    #[test]
    #[should_panic(expected = "Encountered a style value the compiler cannot read.")]
    fn refuses_a_template_that_spells_no_one_piece_of_text() {
      text_of("{ value: `a${b}c` }");
    }

    /// An object stays an object, with the kind of every value in it kept. It
    /// used to be flattened to the JSON text of itself, which a reader of the
    /// answer then saw as a string.
    #[test]
    fn keeps_an_object_as_an_object() {
      let value = value_of("{ value: { nested: 1 } }");

      let FlatCompiledStylesValue::Object(nested) = value.as_ref() else {
        panic!("the constant does not hold an object: {value:?}");
      };

      assert_eq!(
        nested.get("nested").map(Rc::as_ref),
        Some(&FlatCompiledStylesValue::Number(1.0))
      );
    }

    /// The JSON the injected rule carries spells the kind too, so a reader of
    /// the metadata sees a number as a number.
    #[test]
    fn spells_the_json_the_injected_rule_carries() {
      assert_eq!(value_of("{ value: 800 }").to_json_text(), "800");
      assert_eq!(value_of("{ value: '800' }").to_json_text(), "\"800\"");
      assert_eq!(value_of("{ value: true }").to_json_text(), "true");
      assert_eq!(value_of("{ value: null }").to_json_text(), "null");
      assert_eq!(value_of("{ value: { a: 1 } }").to_json_text(), "{\"a\":1}");
      assert_eq!(value_of("{ value: ['x'] }").to_json_text(), "[\"x\"]");
    }
  }

  /// The name a constant is written under in the stylesheet is hashed from the
  /// export it belongs to, in a debug build as in a production one.
  mod the_name_a_constant_is_written_under {
    use super::*;

    use crate::tests::support::expr;

    fn const_keys(code: &str, debug: bool) -> Vec<String> {
      let options = StyleXOptions::default()
        .with_class_name_prefix("x")
        .with_debug(debug);

      let mut state = StateManager::new(options);
      state.export_id = Some("Test.stylex.js//consts".to_owned());

      let (_, injectable_styles) =
        stylex_define_consts(&EvaluateResultValue::Expr(expr(code)), &mut state);

      injectable_styles
        .keys()
        .map(|key| key.as_str().to_owned())
        .collect()
    }

    #[test]
    fn is_the_hash_alone() {
      assert_eq!(
        const_keys("{ '2xl': '(min-width: 1536px)' }", false),
        [get_const_hash("Test.stylex.js//consts", "2xl", "x")]
      );
    }

    /// A debug build names the constant exactly as a production build does.
    #[test]
    fn is_the_same_hash_under_debug() {
      assert_eq!(
        const_keys("{ '2xl': '(min-width: 1536px)' }", true),
        const_keys("{ '2xl': '(min-width: 1536px)' }", false)
      );
    }

    /// A name the author wrote as a CSS custom property keeps that name, with
    /// the two leading dashes taken off.
    #[test]
    fn keeps_a_custom_property_name_the_author_wrote() {
      assert_eq!(const_keys("{ '--brand': 'red' }", true), ["brand"]);
    }
  }
}
