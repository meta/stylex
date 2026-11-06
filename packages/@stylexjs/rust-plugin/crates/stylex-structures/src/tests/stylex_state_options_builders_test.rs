//! Builder coverage tests for `StyleXStateOptions`.

use stylex_enums::style_resolution::StyleResolution;

use crate::{
  named_import_source::{NamedImportSource, RuntimeInjectionState},
  stylex_options::{CheckModuleResolution, ModuleResolution},
  stylex_state_options::StyleXStateOptions,
};

/// `with_*` methods should update core fields and runtime injection state.
#[test]
fn stylex_state_options_builders_update_all_supported_fields() {
  let state = StyleXStateOptions::default()
    .with_class_name_prefix("pref")
    .with_style_resolution(StyleResolution::ApplicationOrder)
    .with_debug(true)
    .with_dev(true)
    .with_test(true)
    .with_enable_debug_data_prop(false)
    .with_enable_dev_class_names(true)
    .with_enable_font_size_px_to_rem(true)
    .with_enable_logical_styles_polyfill(true)
    .with_enable_minified_keys(false)
    .with_unstable_module_resolution(
      ModuleResolution::cross_file_parsing(Some("/repo".to_string())).into(),
    )
    .with_runtime_injection(Some(RuntimeInjectionState::Named(NamedImportSource {
      r#as: "inject".to_string(),
      from: "pkg".to_string(),
    })));

  assert_eq!(state.class_name_prefix, "pref");
  assert_eq!(state.style_resolution, StyleResolution::ApplicationOrder);
  assert!(state.debug);
  assert!(state.dev);
  assert!(state.test);
  assert!(!state.enable_debug_data_prop);
  assert!(state.enable_dev_class_names);
  assert!(state.enable_font_size_px_to_rem);
  assert!(state.enable_logical_styles_polyfill);
  assert!(!state.enable_minified_keys);
  assert!(matches!(
    state.unstable_module_resolution,
    CheckModuleResolution::CrossFileParsing { .. }
  ));
  assert!(matches!(
    state.runtime_injection,
    Some(RuntimeInjectionState::Named(_))
  ));
}

/// `DerefMut` should expose mutable access to the embedded core options.
#[test]
fn stylex_state_options_deref_mut_exposes_core_fields() {
  let state = StyleXStateOptions::default().with_class_name_prefix("mutated");

  assert_eq!(state.core.class_name_prefix, "mutated");
}
