//! Tests for StyleXOptions default values, From<StyleXOptionsParams>
//! conversion, and module resolution factories.

use crate::{
  named_import_source::{NamedImportSource, RuntimeInjection},
  stylex_options::*,
};

#[test]
fn stylex_options_params_default_key_fields() {
  let params = StyleXOptionsParams::default();
  assert_eq!(params.class_name_prefix, Some("x".to_string()));
  assert_eq!(params.dev, Some(false));
  assert_eq!(params.test, Some(false));
  assert!(params.debug.is_none());
  assert_eq!(params.treeshake_compensation, Some(true));
  assert_eq!(params.enable_inlined_conditional_merge, Some(true));
  assert!(params.aliases.is_none());
  assert!(params.unstable_module_resolution.is_none());
}

#[test]
fn stylex_options_default_key_fields() {
  let opts = StyleXOptions::default();
  assert_eq!(opts.class_name_prefix, "x");
  assert!(!opts.dev);
  assert!(!opts.test);
  assert!(!opts.debug);
  assert!(opts.enable_minified_keys);
  assert!(opts.enable_debug_data_prop);
  assert!(!opts.treeshake_compensation);
  assert!(matches!(
    opts.runtime_injection,
    RuntimeInjection::Boolean(false)
  ));
  assert!(matches!(
    opts.unstable_module_resolution,
    CheckModuleResolution::CommonJs { .. }
  ));
  assert_eq!(opts.sx_prop_name, Some("sx".to_string()));
}

#[test]
fn from_stylex_options_params_defaults() {
  let params = StyleXOptionsParams::default();
  let opts: StyleXOptions = params.into();
  assert_eq!(opts.class_name_prefix, "x");
  assert!(!opts.dev);
  assert!(!opts.debug);
  assert!(matches!(
    opts.runtime_injection,
    RuntimeInjection::Boolean(false)
  ));
  assert_eq!(opts.import_sources.len(), 2);
}

#[test]
fn from_stylex_options_params_runtime_injection_true() {
  let params = StyleXOptionsParams {
    runtime_injection: Some(RuntimeInjection::Boolean(true)),
    ..StyleXOptionsParams::default()
  };
  let opts: StyleXOptions = params.into();
  assert!(matches!(
    opts.runtime_injection,
    RuntimeInjection::Regular(_)
  ));
}

#[test]
fn from_stylex_options_params_runtime_injection_named() {
  let params = StyleXOptionsParams {
    runtime_injection: Some(RuntimeInjection::Named(NamedImportSource {
      r#as: "inject".into(),
      from: "my-pkg".into(),
    })),
    ..StyleXOptionsParams::default()
  };
  let opts: StyleXOptions = params.into();
  assert!(matches!(opts.runtime_injection, RuntimeInjection::Named(_)));
}

#[test]
fn from_stylex_options_params_haste_resolution() {
  let params = StyleXOptionsParams {
    unstable_module_resolution: Some(ModuleResolution {
      kind: ModuleResolutionKind::Haste,
      root_dir: Some("/root".to_string()),
      theme_file_extension: None,
    }),
    ..StyleXOptionsParams::default()
  };
  let opts: StyleXOptions = params.into();
  assert!(matches!(
    opts.unstable_module_resolution,
    CheckModuleResolution::Haste { .. }
  ));
}

#[test]
fn from_stylex_options_params_cross_file_parsing_resolution() {
  let params = StyleXOptionsParams {
    unstable_module_resolution: Some(ModuleResolution {
      kind: ModuleResolutionKind::CrossFileParsing,
      root_dir: None,
      theme_file_extension: None,
    }),
    ..StyleXOptionsParams::default()
  };

  let opts: StyleXOptions = params.into();
  assert!(matches!(
    opts.unstable_module_resolution,
    CheckModuleResolution::CrossFileParsing { .. }
  ));
}

#[test]
fn check_module_resolution_accessors_return_variant_values() {
  let cases = [
    CheckModuleResolution::CommonJs {
      root_dir: Some("/common".into()),
      theme_file_extension: Some(".common".into()),
    },
    CheckModuleResolution::Haste {
      root_dir: Some("/haste".into()),
      theme_file_extension: Some(".haste".into()),
    },
    CheckModuleResolution::CrossFileParsing {
      root_dir: Some("/cross".into()),
      theme_file_extension: Some(".cross".into()),
    },
  ];

  assert_eq!(cases[0].root_dir(), Some("/common"));
  assert_eq!(cases[0].theme_file_extension(), Some(".common"));
  assert_eq!(cases[1].root_dir(), Some("/haste"));
  assert_eq!(cases[1].theme_file_extension(), Some(".haste"));
  assert_eq!(cases[2].root_dir(), Some("/cross"));
  assert_eq!(cases[2].theme_file_extension(), Some(".cross"));
}

#[test]
fn get_haste_module_resolution() {
  let res = ModuleResolution::haste(Some("/root".to_string()));
  assert_eq!(res.kind, ModuleResolutionKind::Haste);
  assert_eq!(res.root_dir, Some("/root".to_string()));
  assert!(res.theme_file_extension.is_none());
}

#[test]
fn get_common_js_module_resolution() {
  let res = ModuleResolution::common_js(None);
  assert_eq!(res.kind, ModuleResolutionKind::CommonJs);
  assert!(res.root_dir.is_none());
}

#[test]
fn stylex_options_deref_mut_updates_core() {
  // Exercise DerefMut so mutable access goes through the core options bridge.
  let mut opts = StyleXOptions::default();
  std::ops::DerefMut::deref_mut(&mut opts).class_name_prefix = "mutated".into();
  assert_eq!(opts.class_name_prefix, "mutated");
}
