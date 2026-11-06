//! Builder and conversion edge-case tests for `StyleXOptions`.

use stylex_enums::{style_resolution::StyleResolution, sx_prop_name_param::SxPropNameParam};

use crate::{
  named_import_source::{NamedImportSource, RuntimeInjection},
  stylex_options::{
    CheckModuleResolution, ModuleResolution, ModuleResolutionKind, StyleXOptions,
    StyleXOptionsParams,
  },
};

/// `StyleXOptions` fluent setters should update both core fields and runtime
/// injection.
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

/// Conversion should preserve explicit regular injection and custom `sx` name.
#[test]
fn from_params_handles_regular_runtime_and_custom_sx_name() {
  let params = StyleXOptionsParams {
    runtime_injection: Some(RuntimeInjection::Regular("custom/path".to_string())),
    sx_prop_name: Some(SxPropNameParam::Enabled("sx2".to_string())),
    ..StyleXOptionsParams::default()
  };

  let opts: StyleXOptions = params.into();
  assert!(matches!(
    opts.runtime_injection,
    RuntimeInjection::Regular(_)
  ));
  assert_eq!(opts.sx_prop_name, Some("sx2".to_string()));
}

/// Conversion should disable `sx` prop when explicitly requested.
#[test]
fn from_params_handles_disabled_sx_prop_name() {
  let params = StyleXOptionsParams {
    sx_prop_name: Some(SxPropNameParam::Disabled),
    ..StyleXOptionsParams::default()
  };
  let opts: StyleXOptions = params.into();
  assert_eq!(opts.sx_prop_name, None);
}

/// A name that is blank, or only spaces, names no prop. Upstream compares the
/// name against a JSX attribute name, which is never empty, so a blank name is
/// enabled there and simply never matches. Resolving it to disabled reaches
/// the same outcome on every path, the compiled one included.
#[test]
fn from_params_handles_a_blank_sx_prop_name_as_disabled() {
  for name in ["", " ", "\t\n"] {
    let params = StyleXOptionsParams {
      sx_prop_name: Some(SxPropNameParam::Enabled(name.to_string())),
      ..StyleXOptionsParams::default()
    };

    let opts: StyleXOptions = params.into();

    assert_eq!(opts.sx_prop_name, None, "a name of {name:?} names no prop");
  }
}

/// Module resolution helper constructors should set the correct discriminator.
#[test]
fn module_resolution_helper_builders_have_expected_type() {
  let common = ModuleResolution::common_js(Some("/common".to_string()));
  let haste = ModuleResolution::haste(Some("/haste".to_string()));
  let cross = ModuleResolution::cross_file_parsing(Some("/cross".to_string()));

  assert_eq!(common.kind, ModuleResolutionKind::CommonJs);
  assert_eq!(haste.kind, ModuleResolutionKind::Haste);
  assert_eq!(cross.kind, ModuleResolutionKind::CrossFileParsing);
}

/// Named runtime injection path should survive `StyleXOptionsParams`
/// conversion.
#[test]
fn from_params_keeps_named_runtime_injection() {
  let params = StyleXOptionsParams {
    runtime_injection: Some(RuntimeInjection::Named(NamedImportSource {
      r#as: "inject".to_string(),
      from: "@pkg/stylex".to_string(),
    })),
    ..StyleXOptionsParams::default()
  };

  let opts: StyleXOptions = params.into();
  assert!(matches!(opts.runtime_injection, RuntimeInjection::Named(_)));
}

/// `DerefMut` should expose mutable access to the embedded core options.
#[test]
fn stylex_options_deref_mut_exposes_core_fields() {
  let opts = StyleXOptions::default().with_class_name_prefix("mutated");
  assert_eq!(opts.core.class_name_prefix, "mutated");
}
