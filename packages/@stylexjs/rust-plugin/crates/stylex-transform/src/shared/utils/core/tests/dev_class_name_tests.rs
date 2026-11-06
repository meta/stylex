//! Debug class names a dev build adds to each style namespace.

use super::{dev_class_name, dev_class_name_prefix, file_basename};

fn name(namespace: &str, var_name: Option<&str>, filename: &str) -> String {
  let prefix = dev_class_name_prefix(&var_name.map(str::to_owned), filename);

  dev_class_name(&prefix, namespace)
}

#[test]
fn names_a_namespace_after_its_file_and_variable() {
  assert_eq!(
    name("root", Some("styles"), "MyComponent"),
    "MyComponent__styles.root"
  );
}

#[test]
fn leaves_out_the_variable_when_there_is_none() {
  assert_eq!(name("root", None, "MyComponent"), "MyComponent__root");
}

/// A space would split the name into two classes, the first naming another namespace.
#[test]
fn removes_a_character_that_a_class_name_cannot_hold() {
  assert_eq!(name("p+1", Some("c"), "input"), "input__c.p1");
  assert_eq!(name("a b+c!d", Some("c"), "input"), "input__c.abcd");
}

#[test]
fn keeps_the_characters_a_class_name_can_hold() {
  assert_eq!(name("a-b_c.d", None, "input"), "input__a-b_c.d");
}

/// The name is a label for a person, so a duplicate is acceptable; a stray space is not.
#[test]
fn two_namespaces_can_share_a_name() {
  assert_eq!(
    name("p+1", Some("c"), "input"),
    name("p1", Some("c"), "input")
  );
}

#[test]
fn keeps_only_the_first_part_of_a_file_name_with_several_dots() {
  assert_eq!(name("root", Some("s"), "input.stylex"), "input__s.root");
  assert_eq!(name("root", Some("s"), "a.b.c"), "a__s.root");
}

#[test]
fn reads_the_file_name_out_of_a_path() {
  assert_eq!(
    name("root", Some("s"), "src/components/Card.tsx"),
    "Card__s.root"
  );
}

/// The name still has to mark the file part, or every unknown file gives the same name.
#[test]
fn names_an_unknown_file() {
  assert_eq!(name("base", Some("styles"), ""), "UnknownFile__styles.base");
  assert_eq!(name("base", None, "."), "UnknownFile__base");
  assert_eq!(name("base", None, "/"), "UnknownFile__base");
}

#[test]
fn accepts_a_namespace_that_is_left_empty() {
  assert_eq!(name("+++", Some("c"), "input"), "input__c.");
}

#[test]
fn removes_the_accented_letters_of_a_namespace() {
  assert_eq!(name("ünïcödé", None, "input"), "input__ncd");
}

#[test]
fn accepts_a_very_long_namespace() {
  let namespace = "a".repeat(100_000);

  assert_eq!(
    name(&namespace, Some("c"), "input").len(),
    "input__c.".len() + 100_000
  );
}

#[test]
fn accepts_a_very_long_namespace_that_is_left_empty() {
  let namespace = "+".repeat(100_000);

  assert_eq!(name(&namespace, Some("c"), "input"), "input__c.");
}

#[test]
fn reads_a_base_name_that_needs_no_change() {
  assert_eq!(file_basename("MyComponent"), "MyComponent");
}

mod test_styles {
  use std::rc::Rc;

  use indexmap::IndexMap;
  use stylex_constants::constants::common::COMPILED_KEY;
  use stylex_state::{
    flat_compiled_styles_value::FlatCompiledStylesValue,
    state_manager::StateManager,
    types::{FlatCompiledStyles, StylesObjectMap},
  };
  use stylex_structures::plugin_pass::PluginPass;
  use swc_core::common::FileName;

  use crate::shared::utils::core::dev_class_name::{
    convert_theme_to_dev_styles, convert_theme_to_test_styles, convert_to_test_styles,
  };

  /// The filename is the only part of the state a debug name depends on.
  fn state_of(filename: &str) -> StateManager {
    let mut state = StateManager::default();

    state.set_plugin_pass(PluginPass {
      cwd: None,
      filename: FileName::Real(filename.into()),
    });

    state
  }

  fn styles_of(pairs: &[(&str, &str)]) -> Rc<FlatCompiledStyles> {
    let mut styles: FlatCompiledStyles = IndexMap::new();

    for (key, value) in pairs {
      styles.insert(
        (*key).to_owned(),
        Rc::new(FlatCompiledStylesValue::String((*value).to_owned())),
      );
    }

    Rc::new(styles)
  }

  fn namespaces_of(names: &[&str]) -> StylesObjectMap {
    let mut obj: StylesObjectMap = IndexMap::new();

    for name in names {
      obj.insert((*name).to_owned(), styles_of(&[("color", "xabc")]));
    }

    obj
  }

  fn keys_of(obj: &StylesObjectMap, namespace: &str) -> Vec<String> {
    match obj.get(namespace) {
      Some(styles) => styles.keys().cloned().collect(),
      None => panic!("the namespace {namespace} is not in the answer"),
    }
  }

  /// The element's class is then the name a person reads in the snapshot.
  #[test]
  fn a_test_style_holds_its_name_and_the_marker_only() {
    let result = convert_to_test_styles(
      namespaces_of(&["root"]),
      &Some("styles".to_owned()),
      &state_of("src/Card.tsx"),
    );

    assert_eq!(
      keys_of(&result, "root"),
      ["Card__styles.root", COMPILED_KEY]
    );

    let name = match result["root"]["Card__styles.root"].as_ref() {
      FlatCompiledStylesValue::String(name) => name.clone(),
      other => panic!("the debug name is not a string: {other:?}"),
    };

    assert_eq!(name, "Card__styles.root");
    assert_eq!(
      result["root"][COMPILED_KEY].as_ref(),
      &FlatCompiledStylesValue::Bool(true)
    );
  }

  #[test]
  fn a_test_style_names_every_namespace_in_order() {
    let result = convert_to_test_styles(
      namespaces_of(&["root", "dark", "root2"]),
      &Some("s".to_owned()),
      &state_of("src/Card.tsx"),
    );

    assert_eq!(
      result.keys().cloned().collect::<Vec<_>>(),
      ["root", "dark", "root2"]
    );
    assert_eq!(keys_of(&result, "dark")[0], "Card__s.dark");
    assert_eq!(keys_of(&result, "root2")[0], "Card__s.root2");
  }

  #[test]
  fn a_test_style_of_no_variable_is_named_after_its_file() {
    let result = convert_to_test_styles(namespaces_of(&["root"]), &None, &state_of("src/Card.tsx"));

    assert_eq!(keys_of(&result, "root")[0], "Card__root");
  }

  #[test]
  fn a_test_style_of_no_namespace_is_empty() {
    let result = convert_to_test_styles(
      StylesObjectMap::new(),
      &Some("s".to_owned()),
      &state_of("src/Card.tsx"),
    );

    assert!(result.is_empty());
  }

  /// The name is a label, so the duplicate is acceptable; a lost namespace is not.
  #[test]
  fn two_test_namespaces_can_share_a_name() {
    let result = convert_to_test_styles(
      namespaces_of(&["p+1", "p1"]),
      &Some("c".to_owned()),
      &state_of("input.ts"),
    );

    assert_eq!(result.len(), 2);
    assert_eq!(keys_of(&result, "p+1")[0], "input__c.p1");
    assert_eq!(keys_of(&result, "p1")[0], "input__c.p1");
  }

  #[test]
  fn a_theme_dev_style_puts_its_name_in_front_of_the_overrides() {
    let overrides = styles_of(&[("--color", "red"), ("--size", "1px")]);

    let result =
      convert_theme_to_dev_styles(&Some("buttonTheme".to_owned()), &overrides, "src/Theme.ts");

    assert_eq!(
      result.keys().cloned().collect::<Vec<_>>(),
      ["Theme__buttonTheme", "--color", "--size"]
    );
    assert_eq!(
      result["Theme__buttonTheme"].as_ref(),
      &FlatCompiledStylesValue::String("Theme__buttonTheme".to_owned())
    );
  }

  /// The theme name is not sanitized.
  #[test]
  fn a_theme_dev_style_keeps_the_variable_name_as_it_was_written() {
    let result = convert_theme_to_dev_styles(
      &Some("a+b".to_owned()),
      &FlatCompiledStyles::new(),
      "input.ts",
    );

    assert_eq!(result.keys().cloned().collect::<Vec<_>>(), ["input__a+b"]);
  }

  /// Overrides are put in after the name.
  #[test]
  fn an_override_keyed_by_the_debug_name_wins() {
    let overrides = styles_of(&[("Theme__t", "override")]);

    let result = convert_theme_to_dev_styles(&Some("t".to_owned()), &overrides, "Theme.ts");

    assert_eq!(result.len(), 1);
    assert_eq!(
      result["Theme__t"].as_ref(),
      &FlatCompiledStylesValue::String("override".to_owned())
    );
  }

  /// Refused rather than answered with a name every theme would share.
  #[test]
  #[should_panic(expected = "The variable name could not be determined.")]
  fn a_theme_dev_style_needs_a_variable_name() {
    convert_theme_to_dev_styles(&None, &FlatCompiledStyles::new(), "Theme.ts");
  }

  /// Babel's test branch (unchanged 2d42d72, visitors/stylex-create-theme.js:185-192).
  #[test]
  fn a_theme_test_style_replaces_real_theme_classes_and_overrides() {
    let overrides = styles_of(&[
      ("xgroup", "xreal xgroup"),
      ("Theme__t", "not the label"),
      (COMPILED_KEY, "not a marker"),
    ]);
    let result = convert_theme_to_test_styles(&Some("t".to_owned()), &overrides, "src/Theme.ts");
    assert_eq!(
      result.keys().cloned().collect::<Vec<_>>(),
      ["Theme__t", COMPILED_KEY]
    );
    assert_eq!(
      result["Theme__t"].as_ref(),
      &FlatCompiledStylesValue::String("Theme__t".to_owned())
    );
    assert_eq!(
      result[COMPILED_KEY].as_ref(),
      &FlatCompiledStylesValue::Bool(true)
    );
  }

  #[test]
  fn a_theme_test_style_adds_the_compiled_marker_last() {
    let overrides = styles_of(&[("--color", "red")]);

    let result = convert_theme_to_test_styles(&Some("t".to_owned()), &overrides, "src/Theme.ts");

    assert_eq!(
      result.keys().cloned().collect::<Vec<_>>(),
      ["Theme__t", COMPILED_KEY]
    );
    assert_eq!(
      result[COMPILED_KEY].as_ref(),
      &FlatCompiledStylesValue::Bool(true)
    );
  }

  #[test]
  fn a_theme_test_style_holds_one_compiled_marker() {
    let overrides = styles_of(&[(COMPILED_KEY, "not a marker"), ("--color", "red")]);

    let result = convert_theme_to_test_styles(&Some("t".to_owned()), &overrides, "Theme.ts");

    assert_eq!(
      result.keys().cloned().collect::<Vec<_>>(),
      ["Theme__t", COMPILED_KEY]
    );
    assert_eq!(
      result[COMPILED_KEY].as_ref(),
      &FlatCompiledStylesValue::Bool(true)
    );
  }

  #[test]
  #[should_panic(expected = "The variable name could not be determined.")]
  fn a_theme_test_style_needs_a_variable_name() {
    convert_theme_to_test_styles(&None, &FlatCompiledStyles::new(), "Theme.ts");
  }
}
