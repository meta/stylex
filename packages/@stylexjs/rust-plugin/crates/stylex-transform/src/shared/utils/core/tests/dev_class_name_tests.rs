//! Tests for the debug class names a dev build adds to each style namespace.

use super::{dev_class_name, dev_class_name_prefix, file_basename};

/// Shorthand for the name built from a namespace, a variable name and a file.
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

/// A class name holds one class. A space in the name would make it two, and the
/// first of the two is the name of another namespace of the same variable.
#[test]
fn removes_a_character_that_a_class_name_cannot_hold() {
  assert_eq!(name("p+1", Some("c"), "input"), "input__c.p1");
  assert_eq!(name("a b+c!d", Some("c"), "input"), "input__c.abcd");
}

#[test]
fn keeps_the_characters_a_class_name_can_hold() {
  assert_eq!(name("a-b_c.d", None, "input"), "input__a-b_c.d");
}

/// Two namespaces that differ only in a character no class name can hold get
/// the same debug name. The name is a label for a person to read, so a
/// duplicate is acceptable; a space, which silently adds a second class, is
/// not.
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

/// The compiler does not always know the file. The name still has to say which
/// part is the file, or every file gives the same name.
#[test]
fn names_an_unknown_file() {
  assert_eq!(name("base", Some("styles"), ""), "UnknownFile__styles.base");
  assert_eq!(name("base", None, "."), "UnknownFile__base");
  assert_eq!(name("base", None, "/"), "UnknownFile__base");
}

/// Every character of the namespace can be one a class name cannot hold. The
/// name then says the file and the variable and nothing more.
#[test]
fn accepts_a_namespace_that_is_left_empty() {
  assert_eq!(name("+++", Some("c"), "input"), "input__c.");
}

/// A class name holds no letter outside the ASCII range, so an accented letter
/// goes and the plain letters beside it stay.
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

/// Every character is one to remove, so the work is a full rewrite of a large
/// string rather than a copy of it.
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

  /// A state that reads the given file, which is the only part of the state a
  /// debug name depends on.
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

  /// The keys of a namespace, in the order the map holds them.
  fn keys_of(obj: &StylesObjectMap, namespace: &str) -> Vec<String> {
    match obj.get(namespace) {
      Some(styles) => styles.keys().cloned().collect(),
      None => panic!("the namespace {namespace} is not in the answer"),
    }
  }

  /// A test build keeps the debug name and the compiled marker, and drops every
  /// style the namespace held. The class the element gets is then the name a
  /// person reads in the snapshot.
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

  /// Every namespace is named for itself, and the namespaces keep the order the
  /// author wrote them in.
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

  /// Styles bound to no variable still get a name, and it says the file only.
  #[test]
  fn a_test_style_of_no_variable_is_named_after_its_file() {
    let result = convert_to_test_styles(namespaces_of(&["root"]), &None, &state_of("src/Card.tsx"));

    assert_eq!(keys_of(&result, "root")[0], "Card__root");
  }

  /// No namespace is no work, and the answer is an empty map rather than a
  /// map holding an empty namespace.
  #[test]
  fn a_test_style_of_no_namespace_is_empty() {
    let result = convert_to_test_styles(
      StylesObjectMap::new(),
      &Some("s".to_owned()),
      &state_of("src/Card.tsx"),
    );

    assert!(result.is_empty());
  }

  /// Two namespaces that share a debug name keep their own entries. The name is
  /// a label, so the duplicate is acceptable; a lost namespace is not.
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

  /// The theme name is not sanitized, so a variable holding a character no
  /// class name can hold reaches the answer as it was written.
  #[test]
  fn a_theme_dev_style_keeps_the_variable_name_as_it_was_written() {
    let result = convert_theme_to_dev_styles(
      &Some("a+b".to_owned()),
      &FlatCompiledStyles::new(),
      "input.ts",
    );

    assert_eq!(result.keys().cloned().collect::<Vec<_>>(), ["input__a+b"]);
  }

  /// An override that is keyed by the debug name replaces it, because the
  /// overrides are put in after the name.
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

  /// A theme is always bound to a variable. Without one there is no name to
  /// build, so the call is refused rather than answered with a name every theme
  /// would share.
  #[test]
  #[should_panic(expected = "The variable name could not be determined.")]
  fn a_theme_dev_style_needs_a_variable_name() {
    convert_theme_to_dev_styles(&None, &FlatCompiledStyles::new(), "Theme.ts");
  }

  /// A test theme is the development theme with the compiled marker at the end.
  #[test]
  fn a_theme_test_style_adds_the_compiled_marker_last() {
    let overrides = styles_of(&[("--color", "red")]);

    let result = convert_theme_to_test_styles(&Some("t".to_owned()), &overrides, "src/Theme.ts");

    assert_eq!(
      result.keys().cloned().collect::<Vec<_>>(),
      ["Theme__t", "--color", COMPILED_KEY]
    );
    assert_eq!(
      result[COMPILED_KEY].as_ref(),
      &FlatCompiledStylesValue::Bool(true)
    );
  }

  /// The marker is one key. An override already holding it is replaced, and the
  /// key keeps the place the override gave it.
  #[test]
  fn a_theme_test_style_holds_one_compiled_marker() {
    let overrides = styles_of(&[(COMPILED_KEY, "not a marker"), ("--color", "red")]);

    let result = convert_theme_to_test_styles(&Some("t".to_owned()), &overrides, "Theme.ts");

    assert_eq!(
      result.keys().cloned().collect::<Vec<_>>(),
      ["Theme__t", COMPILED_KEY, "--color"]
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
