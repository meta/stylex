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

/// Direct `with_*` setters should mutate all targeted fields.
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

/// Optional `maybe_*` setters with `None` must leave the field unchanged.
#[test]
fn core_stylex_options_optional_setters_skip_none_values() {
  let original = CoreStyleXOptions::default();
  let opts = CoreStyleXOptions::default()
    .maybe_dev(None)
    .maybe_test(None)
    .maybe_style_resolution(None)
    .maybe_property_validation_mode(None)
    .maybe_enable_font_size_px_to_rem(None)
    .maybe_class_name_prefix(None)
    .maybe_enable_debug_data_prop(None)
    .maybe_enable_dev_class_names(None)
    .maybe_enable_minified_keys(None)
    .maybe_inject_stylex_side_effects(None)
    .maybe_treeshake_compensation(None)
    .maybe_enable_inlined_conditional_merge(None)
    .maybe_enable_media_query_order(None)
    .maybe_enable_logical_styles_polyfill(None)
    .maybe_enable_legacy_value_flipping(None)
    .maybe_enable_ltr_rtl_comments(None)
    .maybe_use_real_file_for_source(None);

  assert_eq!(opts.dev, original.dev);
  assert_eq!(opts.test, original.test);
  assert_eq!(opts.style_resolution, original.style_resolution);
  assert_eq!(
    opts.property_validation_mode,
    original.property_validation_mode
  );
  assert_eq!(
    opts.enable_font_size_px_to_rem,
    original.enable_font_size_px_to_rem
  );
  assert_eq!(opts.class_name_prefix, original.class_name_prefix);
  assert_eq!(opts.enable_debug_data_prop, original.enable_debug_data_prop);
  assert_eq!(opts.enable_dev_class_names, original.enable_dev_class_names);
  assert_eq!(opts.enable_minified_keys, original.enable_minified_keys);
  assert_eq!(
    opts.inject_stylex_side_effects,
    original.inject_stylex_side_effects
  );
  assert_eq!(opts.treeshake_compensation, original.treeshake_compensation);
  assert_eq!(
    opts.enable_inlined_conditional_merge,
    original.enable_inlined_conditional_merge
  );
  assert_eq!(
    opts.enable_media_query_order,
    original.enable_media_query_order
  );
  assert_eq!(
    opts.enable_logical_styles_polyfill,
    original.enable_logical_styles_polyfill
  );
  assert_eq!(
    opts.enable_legacy_value_flipping,
    original.enable_legacy_value_flipping
  );
  assert_eq!(
    opts.enable_ltr_rtl_comments,
    original.enable_ltr_rtl_comments
  );
  assert_eq!(
    opts.use_real_file_for_source,
    original.use_real_file_for_source
  );
}

/// Optional `maybe_*` setters should apply values only when `Some`.
#[test]
fn core_stylex_options_optional_setters_apply_some_values() {
  let opts = CoreStyleXOptions::default()
    .maybe_dev(Some(true))
    .maybe_test(Some(true))
    .maybe_style_resolution(Some(StyleResolution::ApplicationOrder))
    .maybe_property_validation_mode(Some(PropertyValidationMode::Throw))
    .maybe_enable_font_size_px_to_rem(Some(true))
    .maybe_class_name_prefix(Some("abc".to_string()))
    .maybe_enable_debug_data_prop(Some(false))
    .maybe_enable_dev_class_names(Some(true))
    .maybe_enable_minified_keys(Some(false))
    .maybe_inject_stylex_side_effects(Some(true))
    .maybe_treeshake_compensation(Some(true))
    .maybe_enable_inlined_conditional_merge(Some(false))
    .maybe_enable_media_query_order(Some(false))
    .maybe_enable_logical_styles_polyfill(Some(true))
    .maybe_enable_legacy_value_flipping(Some(true))
    .maybe_enable_ltr_rtl_comments(Some(true))
    .maybe_use_real_file_for_source(Some(false));

  assert!(opts.dev);
  assert!(opts.test);
  assert_eq!(opts.style_resolution, StyleResolution::ApplicationOrder);
  assert_eq!(opts.property_validation_mode, PropertyValidationMode::Throw);
  assert!(opts.enable_font_size_px_to_rem);
  assert_eq!(opts.class_name_prefix, "abc");
  assert!(!opts.enable_debug_data_prop);
  assert!(opts.enable_dev_class_names);
  assert!(!opts.enable_minified_keys);
  assert!(opts.inject_stylex_side_effects);
  assert!(opts.treeshake_compensation);
  assert!(!opts.enable_inlined_conditional_merge);
  assert!(!opts.enable_media_query_order);
  assert!(opts.enable_logical_styles_polyfill);
  assert!(opts.enable_legacy_value_flipping);
  assert!(opts.enable_ltr_rtl_comments);
  assert!(!opts.use_real_file_for_source);
}
