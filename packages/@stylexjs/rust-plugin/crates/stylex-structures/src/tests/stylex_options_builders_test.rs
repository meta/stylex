//! Builder and conversion edge-case tests for `StyleXOptions`.

use stylex_enums::style_resolution::StyleResolution;

use crate::{
  named_import_source::RuntimeInjection,
  stylex_options::{CheckModuleResolution, ModuleResolution, ModuleResolutionKind, StyleXOptions},
};

#[test]
fn stylex_options_builders_update_supported_fields() {
  let opts = StyleXOptions::default()
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
    .with_runtime_injection(RuntimeInjection::Regular("/custom/inject".to_string()))
    .with_unstable_module_resolution(ModuleResolution::haste(Some("/repo".to_string())).into());

  assert_eq!(opts.class_name_prefix, "pref");
  assert_eq!(opts.style_resolution, StyleResolution::ApplicationOrder);
  assert!(opts.debug);
  assert!(opts.dev);
  assert!(opts.test);
  assert!(!opts.enable_debug_data_prop);
  assert!(opts.enable_dev_class_names);
  assert!(opts.enable_font_size_px_to_rem);
  assert!(opts.enable_logical_styles_polyfill);
  assert!(!opts.enable_minified_keys);
  assert!(matches!(
    opts.runtime_injection,
    RuntimeInjection::Regular(_)
  ));
  assert!(matches!(
    opts.unstable_module_resolution,
    CheckModuleResolution::Haste { .. }
  ));
}

#[test]
fn module_resolution_helper_builders_have_expected_type() {
  let common = ModuleResolution::common_js(Some("/common".to_string()));
  let haste = ModuleResolution::haste(Some("/haste".to_string()));
  let cross = ModuleResolution::cross_file_parsing(Some("/cross".to_string()));

  assert_eq!(common.kind, ModuleResolutionKind::CommonJs);
  assert_eq!(haste.kind, ModuleResolutionKind::Haste);
  assert_eq!(cross.kind, ModuleResolutionKind::CrossFileParsing);
}
