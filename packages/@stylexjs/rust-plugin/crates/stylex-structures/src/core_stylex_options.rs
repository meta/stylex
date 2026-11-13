use std::rc::Rc;

use indexmap::{IndexMap, IndexSet};
use rustc_hash::FxHashMap;
use serde::Deserialize;

use stylex_enums::{
  property_validation_mode::PropertyValidationMode, style_resolution::StyleResolution,
};

use crate::{
  evaluation_depth::MAX_EVALUATION_DEPTH,
  fold_ceilings::{MAX_FOLDED_CHARACTERS, MAX_FOLDED_ENTRIES},
  import_resolver::{DebugFileNamer, ImportResolver},
  named_import_source::ImportSources,
  stylex_env::{EnvEntry, HostEnvironment, JSFunction},
  stylex_options::CheckModuleResolution,
};

/// Fields shared by `StyleXOptions` and `StyleXStateOptions`, exposed through their `Deref`.
#[derive(Deserialize, Clone, Debug)]
pub struct CoreStyleXOptions {
  pub dev: bool,
  pub test: bool,
  pub debug: bool,
  pub property_validation_mode: PropertyValidationMode,
  pub enable_debug_data_prop: bool,
  pub enable_dev_class_names: bool,
  pub enable_inlined_conditional_merge: bool,
  pub enable_media_query_order: bool,
  pub enable_logical_styles_polyfill: bool,
  pub enable_legacy_value_flipping: bool,
  // Configured by callers but not yet read by the transform.
  #[allow(dead_code)]
  pub enable_ltr_rtl_comments: bool,
  pub enable_minified_keys: bool,
  pub enable_font_size_px_to_rem: bool,
  pub use_real_file_for_source: bool,
  pub class_name_prefix: String,
  pub style_resolution: StyleResolution,
  /// Import sources in fixed order (`[@stylexjs/stylex, stylex, ...configured]`), deduplicated;
  /// first configured wins.
  pub import_sources: IndexSet<ImportSources>,
  pub treeshake_compensation: bool,
  pub inject_stylex_side_effects: bool,
  pub aliases: Option<FxHashMap<String, Vec<String>>>,
  pub unstable_module_resolution: CheckModuleResolution,
  pub sx_prop_name: Option<String>,
  /// Evaluator depth ceiling, already resolved (see [`crate::evaluation_depth`]).
  pub max_evaluation_depth: usize,
  /// String ceiling per fold, already resolved (see [`crate::fold_ceilings`]).
  pub max_folded_characters: usize,
  /// Entry ceiling per fold, already resolved (see [`crate::fold_ceilings`]).
  pub max_folded_entries: usize,
  /// The `env` option's object, shared rather than copied; read-only once options are built.
  #[serde(skip)]
  pub env: Rc<IndexMap<String, EnvEntry>>,
  #[serde(skip)]
  pub debug_file_path: Option<JSFunction>,
  /// See `StyleXOptionsParams::host_environment`.
  #[serde(skip)]
  pub host_environment: Option<HostEnvironment>,
  /// See `StyleXOptionsParams::import_resolver`.
  #[serde(skip)]
  pub import_resolver: Option<Rc<dyn ImportResolver>>,
  /// Names files in debug labels through the host's `debugFilePath`.
  #[serde(skip)]
  pub debug_file_namer: Option<Rc<dyn DebugFileNamer>>,
}

impl Default for CoreStyleXOptions {
  fn default() -> Self {
    CoreStyleXOptions {
      dev: false,
      test: false,
      debug: false,
      property_validation_mode: PropertyValidationMode::Silent,
      enable_debug_data_prop: true,
      enable_dev_class_names: false,
      enable_inlined_conditional_merge: true,
      enable_media_query_order: true,
      enable_logical_styles_polyfill: false,
      enable_legacy_value_flipping: false,
      enable_ltr_rtl_comments: false,
      enable_minified_keys: true,
      enable_font_size_px_to_rem: false,
      use_real_file_for_source: true,
      class_name_prefix: "x".to_string(),
      style_resolution: StyleResolution::PropertySpecificity,
      import_sources: IndexSet::new(),
      treeshake_compensation: false,
      inject_stylex_side_effects: false,
      aliases: None,
      unstable_module_resolution: CheckModuleResolution::default(),
      sx_prop_name: Some("sx".to_string()),
      max_evaluation_depth: MAX_EVALUATION_DEPTH.resolve(None),
      max_folded_characters: MAX_FOLDED_CHARACTERS.resolve(None),
      max_folded_entries: MAX_FOLDED_ENTRIES.resolve(None),
      env: Rc::new(IndexMap::new()),
      debug_file_path: None,
      host_environment: None,
      import_resolver: None,
      debug_file_namer: None,
    }
  }
}

/// Builder methods for `CoreStyleXOptions`.
///
/// `with_*` methods set a field unconditionally.
/// `maybe_*` methods set a field only when the `Option` is `Some`.
impl CoreStyleXOptions {
  // Direct setters.

  pub fn with_dev(mut self, dev: bool) -> Self {
    self.dev = dev;
    self
  }

  pub fn with_test(mut self, test: bool) -> Self {
    self.test = test;
    self
  }

  pub fn with_debug(mut self, debug: bool) -> Self {
    self.debug = debug;
    self
  }

  pub fn with_property_validation_mode(mut self, mode: PropertyValidationMode) -> Self {
    self.property_validation_mode = mode;
    self
  }

  pub fn with_enable_debug_data_prop(mut self, enabled: bool) -> Self {
    self.enable_debug_data_prop = enabled;
    self
  }

  pub fn with_enable_dev_class_names(mut self, enabled: bool) -> Self {
    self.enable_dev_class_names = enabled;
    self
  }

  pub fn with_enable_inlined_conditional_merge(mut self, enabled: bool) -> Self {
    self.enable_inlined_conditional_merge = enabled;
    self
  }

  pub fn with_enable_media_query_order(mut self, enabled: bool) -> Self {
    self.enable_media_query_order = enabled;
    self
  }

  pub fn with_enable_logical_styles_polyfill(mut self, enabled: bool) -> Self {
    self.enable_logical_styles_polyfill = enabled;
    self
  }

  pub fn with_enable_legacy_value_flipping(mut self, enabled: bool) -> Self {
    self.enable_legacy_value_flipping = enabled;
    self
  }

  pub fn with_enable_ltr_rtl_comments(mut self, enabled: bool) -> Self {
    self.enable_ltr_rtl_comments = enabled;
    self
  }

  /// Set the evaluator's ceiling; `None` falls back to the environment, then the default.
  pub fn maybe_max_evaluation_depth(mut self, depth: Option<usize>) -> Self {
    self.max_evaluation_depth = MAX_EVALUATION_DEPTH.resolve(depth);
    self
  }

  /// Set the fold's string ceiling, resolved the same way.
  pub fn maybe_max_folded_characters(mut self, characters: Option<usize>) -> Self {
    self.max_folded_characters = MAX_FOLDED_CHARACTERS.resolve(characters);
    self
  }

  /// Set the fold's entry ceiling, resolved the same way.
  pub fn maybe_max_folded_entries(mut self, entries: Option<usize>) -> Self {
    self.max_folded_entries = MAX_FOLDED_ENTRIES.resolve(entries);
    self
  }

  pub fn with_enable_minified_keys(mut self, enabled: bool) -> Self {
    self.enable_minified_keys = enabled;
    self
  }

  pub fn with_enable_font_size_px_to_rem(mut self, enabled: bool) -> Self {
    self.enable_font_size_px_to_rem = enabled;
    self
  }

  pub fn with_use_real_file_for_source(mut self, enabled: bool) -> Self {
    self.use_real_file_for_source = enabled;
    self
  }

  pub fn with_class_name_prefix(mut self, prefix: impl Into<String>) -> Self {
    self.class_name_prefix = prefix.into();
    self
  }

  pub fn with_style_resolution(mut self, resolution: StyleResolution) -> Self {
    self.style_resolution = resolution;
    self
  }

  pub fn with_import_sources(mut self, sources: Vec<ImportSources>) -> Self {
    self.import_sources = sources.into_iter().collect();
    self
  }

  pub fn with_treeshake_compensation(mut self, enabled: bool) -> Self {
    self.treeshake_compensation = enabled;
    self
  }

  pub fn with_inject_stylex_side_effects(mut self, enabled: bool) -> Self {
    self.inject_stylex_side_effects = enabled;
    self
  }

  pub fn with_aliases(mut self, aliases: Option<FxHashMap<String, Vec<String>>>) -> Self {
    self.aliases = aliases;
    self
  }

  pub fn with_unstable_module_resolution(mut self, resolution: CheckModuleResolution) -> Self {
    self.unstable_module_resolution = resolution;
    self
  }

  pub fn with_sx_prop_name(mut self, name: Option<String>) -> Self {
    self.sx_prop_name = name;
    self
  }

  pub fn with_env(mut self, env: IndexMap<String, EnvEntry>) -> Self {
    self.env = Rc::new(env);
    self
  }

  pub fn with_debug_file_path(mut self, path: Option<JSFunction>) -> Self {
    self.debug_file_path = path;
    self
  }

  pub fn with_host_environment(mut self, host: Option<HostEnvironment>) -> Self {
    self.host_environment = host;
    self
  }

  pub fn with_import_resolver(mut self, resolver: Option<Rc<dyn ImportResolver>>) -> Self {
    self.import_resolver = resolver;
    self
  }

  pub fn with_debug_file_namer(mut self, namer: Option<Rc<dyn DebugFileNamer>>) -> Self {
    self.debug_file_namer = namer;
    self
  }

  // Optional setters.

  pub fn maybe_dev(mut self, dev: Option<bool>) -> Self {
    if let Some(v) = dev {
      self.dev = v;
    }
    self
  }

  pub fn maybe_test(mut self, test: Option<bool>) -> Self {
    if let Some(v) = test {
      self.test = v;
    }
    self
  }

  pub fn maybe_style_resolution(mut self, resolution: Option<StyleResolution>) -> Self {
    if let Some(v) = resolution {
      self.style_resolution = v;
    }
    self
  }

  pub fn maybe_property_validation_mode(mut self, mode: Option<PropertyValidationMode>) -> Self {
    if let Some(v) = mode {
      self.property_validation_mode = v;
    }
    self
  }

  pub fn maybe_enable_font_size_px_to_rem(mut self, enabled: Option<bool>) -> Self {
    if let Some(v) = enabled {
      self.enable_font_size_px_to_rem = v;
    }
    self
  }

  pub fn maybe_class_name_prefix(mut self, prefix: Option<String>) -> Self {
    if let Some(v) = prefix {
      self.class_name_prefix = v;
    }
    self
  }

  pub fn maybe_enable_debug_data_prop(mut self, enabled: Option<bool>) -> Self {
    if let Some(v) = enabled {
      self.enable_debug_data_prop = v;
    }
    self
  }

  pub fn maybe_enable_dev_class_names(mut self, enabled: Option<bool>) -> Self {
    if let Some(v) = enabled {
      self.enable_dev_class_names = v;
    }
    self
  }

  pub fn maybe_enable_minified_keys(mut self, enabled: Option<bool>) -> Self {
    if let Some(v) = enabled {
      self.enable_minified_keys = v;
    }
    self
  }

  pub fn maybe_inject_stylex_side_effects(mut self, enabled: Option<bool>) -> Self {
    if let Some(v) = enabled {
      self.inject_stylex_side_effects = v;
    }
    self
  }

  pub fn maybe_treeshake_compensation(mut self, enabled: Option<bool>) -> Self {
    if let Some(v) = enabled {
      self.treeshake_compensation = v;
    }
    self
  }

  pub fn maybe_enable_inlined_conditional_merge(mut self, enabled: Option<bool>) -> Self {
    if let Some(v) = enabled {
      self.enable_inlined_conditional_merge = v;
    }
    self
  }

  pub fn maybe_enable_media_query_order(mut self, enabled: Option<bool>) -> Self {
    if let Some(v) = enabled {
      self.enable_media_query_order = v;
    }
    self
  }

  pub fn maybe_enable_logical_styles_polyfill(mut self, enabled: Option<bool>) -> Self {
    if let Some(v) = enabled {
      self.enable_logical_styles_polyfill = v;
    }
    self
  }

  pub fn maybe_enable_legacy_value_flipping(mut self, enabled: Option<bool>) -> Self {
    if let Some(v) = enabled {
      self.enable_legacy_value_flipping = v;
    }
    self
  }

  pub fn maybe_enable_ltr_rtl_comments(mut self, enabled: Option<bool>) -> Self {
    if let Some(v) = enabled {
      self.enable_ltr_rtl_comments = v;
    }
    self
  }

  pub fn maybe_use_real_file_for_source(mut self, enabled: Option<bool>) -> Self {
    if let Some(v) = enabled {
      self.use_real_file_for_source = v;
    }
    self
  }
}

#[cfg(test)]
#[path = "tests/core_stylex_options_test.rs"]
mod tests;
