//! Exhaustive builder tests for `CoreStyleXOptions`.

use indexmap::IndexMap;
use rustc_hash::FxHashMap;
use stylex_enums::{
  property_validation_mode::PropertyValidationMode, style_resolution::StyleResolution,
};
use swc_core::ecma::ast::{Expr, Lit};

use crate::{
  core_stylex_options::CoreStyleXOptions,
  named_import_source::{ImportSources, NamedImportSource},
  stylex_env::{EnvEntry, JSFunction},
  stylex_options::{CheckModuleResolution, ModuleResolution},
};

#[test]
fn core_stylex_options_direct_setters_cover_all_fields() {
  let mut aliases = FxHashMap::default();
  aliases.insert("foo".to_string(), vec!["bar".to_string()]);

  let mut env = IndexMap::new();
  env.insert(
    "token".to_string(),
    EnvEntry::Expr(Expr::Lit(Lit::from("value"))),
  );

  let debug_file_path = JSFunction::new(|_| Expr::Lit(Lit::from("debug-path")));
  let import_sources = vec![
    ImportSources::Regular("stylex".to_string()),
    ImportSources::Named(NamedImportSource {
      r#as: "sx".to_string(),
      from: "@stylexjs/stylex".to_string(),
    }),
  ];

  let opts = CoreStyleXOptions::default()
    .with_dev(true)
    .with_test(true)
    .with_debug(true)
    .with_property_validation_mode(PropertyValidationMode::Warn)
    .with_enable_debug_data_prop(false)
    .with_enable_dev_class_names(true)
    .with_enable_inlined_conditional_merge(false)
    .with_enable_media_query_order(false)
    .with_enable_logical_styles_polyfill(true)
    .with_enable_legacy_value_flipping(true)
    .with_enable_ltr_rtl_comments(true)
    .with_enable_minified_keys(false)
    .with_enable_font_size_px_to_rem(true)
    .with_use_real_file_for_source(false)
    .with_class_name_prefix("pref")
    .with_style_resolution(StyleResolution::ApplicationOrder)
    .with_import_sources(import_sources.clone())
    .with_treeshake_compensation(true)
    .with_inject_stylex_side_effects(true)
    .with_aliases(Some(aliases.clone()))
    .with_unstable_module_resolution(ModuleResolution::haste(Some("/repo".to_string())).into())
    .with_sx_prop_name(None)
    .with_env(env.clone())
    .with_debug_file_path(Some(debug_file_path));

  assert!(opts.dev);
  assert!(opts.test);
  assert!(opts.debug);
  assert_eq!(opts.property_validation_mode, PropertyValidationMode::Warn);
  assert!(!opts.enable_debug_data_prop);
  assert!(opts.enable_dev_class_names);
  assert!(!opts.enable_inlined_conditional_merge);
  assert!(!opts.enable_media_query_order);
  assert!(opts.enable_logical_styles_polyfill);
  assert!(opts.enable_legacy_value_flipping);
  assert!(opts.enable_ltr_rtl_comments);
  assert!(!opts.enable_minified_keys);
  assert!(opts.enable_font_size_px_to_rem);
  assert!(!opts.use_real_file_for_source);
  assert_eq!(opts.class_name_prefix, "pref");
  assert_eq!(opts.style_resolution, StyleResolution::ApplicationOrder);
  assert_eq!(opts.import_sources.len(), import_sources.len());
  assert!(opts.treeshake_compensation);
  assert!(opts.inject_stylex_side_effects);
  assert_eq!(opts.aliases, Some(aliases));
  assert!(matches!(
    opts.unstable_module_resolution,
    CheckModuleResolution::Haste { .. }
  ));
  assert_eq!(opts.sx_prop_name, None);
  assert_eq!(opts.env.len(), env.len());
  assert!(opts.debug_file_path.is_some());
}
