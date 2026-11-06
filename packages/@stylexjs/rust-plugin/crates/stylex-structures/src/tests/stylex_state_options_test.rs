//! Tests for StyleXStateOptions default values and From<StyleXOptions>
//! conversion.

use crate::{
  named_import_source::{NamedImportSource, RuntimeInjection, RuntimeInjectionState},
  stylex_options::{CheckModuleResolution, StyleXOptions},
  stylex_state_options::*,
};

#[test]
fn stylex_state_options_default() {
  let opts = StyleXStateOptions::default();
  assert!(!opts.dev);
  assert!(!opts.test);
  assert!(!opts.debug);
  assert_eq!(opts.class_name_prefix, "x");
  assert!(opts.enable_minified_keys);
  assert!(opts.enable_debug_data_prop);
  assert!(opts.runtime_injection.is_none());
  assert_eq!(opts.sx_prop_name, Some("sx".to_string()));
}

#[test]
fn check_module_resolution_default_is_commonjs() {
  let res = CheckModuleResolution::default();
  assert!(matches!(res, CheckModuleResolution::CommonJs { .. }));
}

#[test]
fn from_stylex_options_boolean_false_injection() {
  let opts = StyleXOptions {
    runtime_injection: RuntimeInjection::Boolean(false),
    ..StyleXOptions::default()
  };
  let state: StyleXStateOptions = opts.into();
  assert!(state.runtime_injection.is_none());
}

#[test]
fn from_stylex_options_boolean_true_injection() {
  let opts = StyleXOptions {
    runtime_injection: RuntimeInjection::Boolean(true),
    ..StyleXOptions::default()
  };
  let state: StyleXStateOptions = opts.into();
  assert!(matches!(
    state.runtime_injection,
    Some(RuntimeInjectionState::Regular(_))
  ));
}

#[test]
fn from_stylex_options_regular_injection() {
  let opts = StyleXOptions {
    runtime_injection: RuntimeInjection::Regular("custom/path".into()),
    ..StyleXOptions::default()
  };
  let state: StyleXStateOptions = opts.into();
  match state.runtime_injection {
    Some(RuntimeInjectionState::Regular(s)) => assert_eq!(s, "custom/path"),
    _ => panic!("Expected Regular injection"),
  }
}

#[test]
fn from_stylex_options_named_injection() {
  let named = NamedImportSource {
    r#as: "inject".into(),
    from: "pkg".into(),
  };
  let opts = StyleXOptions {
    runtime_injection: RuntimeInjection::Named(named),
    ..StyleXOptions::default()
  };
  let state: StyleXStateOptions = opts.into();
  assert!(matches!(
    state.runtime_injection,
    Some(RuntimeInjectionState::Named(_))
  ));
}

#[test]
fn from_stylex_options_preserves_fields() {
  let opts = StyleXOptions::default()
    .with_dev(true)
    .with_test(true)
    .with_debug(true)
    .with_class_name_prefix("y");
  let state: StyleXStateOptions = opts.into();
  assert!(state.dev);
  assert!(state.test);
  assert!(state.debug);
  assert_eq!(state.class_name_prefix, "y");
}

#[test]
fn stylex_state_options_deref_mut_updates_core() {
  // Ensure mutable deref reaches the embedded CoreStyleXOptions value.
  let mut opts = StyleXStateOptions::default();
  std::ops::DerefMut::deref_mut(&mut opts).class_name_prefix = "state-mutated".into();
  assert_eq!(opts.class_name_prefix, "state-mutated");
}

/// The shared default is the default, and it is one value: a reader that keeps
/// the address it was handed sees the same address on the next read.
#[test]
fn the_shared_default_options_are_one_value() {
  let first = with_default_options(|options| options as *const StyleXStateOptions);
  let second = with_default_options(|options| options as *const StyleXStateOptions);

  assert_eq!(first, second);

  with_default_options(|options| {
    assert_eq!(options.class_name_prefix, "x");
    assert!(!options.dev);
    assert!(options.import_sources.is_empty());
  });
}
