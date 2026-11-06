use std::ops::{Deref, DerefMut};

use indexmap::IndexMap;
use rustc_hash::FxHashMap;
use serde::Deserialize;

use stylex_constants::constants::api_names::STYLEX_SX;
use stylex_constants::constants::common::DEFAULT_INJECT_PATH;
use stylex_enums::{
  property_validation_mode::PropertyValidationMode, style_resolution::StyleResolution,
  sx_prop_name_param::SxPropNameParam,
};

use crate::{
  core_stylex_options::CoreStyleXOptions,
  named_import_source::{ImportSources, RuntimeInjection},
  stylex_env::{EnvEntry, JSFunction},
};

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StyleXOptionsParams {
  pub style_resolution: Option<StyleResolution>,
  pub property_validation_mode: Option<PropertyValidationMode>,
  pub enable_font_size_px_to_rem: Option<bool>,
  pub runtime_injection: Option<RuntimeInjection>,
  pub class_name_prefix: Option<String>,
  pub defined_stylex_css_variables: Option<FxHashMap<String, String>>,
  pub import_sources: Option<Vec<ImportSources>>,
  pub treeshake_compensation: Option<bool>,
  pub enable_inlined_conditional_merge: Option<bool>,
  pub enable_media_query_order: Option<bool>,
  pub enable_logical_styles_polyfill: Option<bool>,
  pub enable_legacy_value_flipping: Option<bool>,
  pub enable_ltr_rtl_comments: Option<bool>,
  pub use_real_file_for_source: Option<bool>,
  pub dev: Option<bool>,
  pub test: Option<bool>,
  pub debug: Option<bool>,
  pub enable_debug_data_prop: Option<bool>,
  pub enable_dev_class_names: Option<bool>,
  pub enable_minified_keys: Option<bool>,
  pub inject_stylex_side_effects: Option<bool>,
  pub aliases: Option<FxHashMap<String, Vec<String>>>,
  #[serde(rename = "unstable_moduleResolution")]
  pub unstable_module_resolution: Option<ModuleResolution>,
  pub sx_prop_name: Option<SxPropNameParam>,
  /// How many levels the evaluator may descend before refusing to fold, as
  /// `maxEvaluationDepth`. Absent means the environment decides, and failing
  /// that the built-in default -- see
  /// [`crate::evaluation_depth`].
  pub max_evaluation_depth: Option<usize>,
  /// How many UTF-16 code units of string one fold may build or carry, as
  /// `maxFoldedCharacters`. Absent resolves the same way -- see
  /// [`crate::fold_ceilings`].
  pub max_folded_characters: Option<usize>,
  /// How many array elements and object properties one fold may build or carry,
  /// as `maxFoldedEntries`. Absent resolves the same way -- see
  /// [`crate::fold_ceilings`].
  pub max_folded_entries: Option<usize>,
  #[serde(skip)]
  pub env: Option<IndexMap<String, EnvEntry>>,
  #[serde(skip)]
  pub debug_file_path: Option<JSFunction>,
}

impl Default for StyleXOptionsParams {
  fn default() -> Self {
    StyleXOptionsParams {
      style_resolution: Some(StyleResolution::PropertySpecificity),
      property_validation_mode: Some(PropertyValidationMode::Silent),
      enable_font_size_px_to_rem: Some(false),
      runtime_injection: Some(RuntimeInjection::Boolean(false)),
      class_name_prefix: Some("x".to_string()),
      defined_stylex_css_variables: Some(FxHashMap::default()),
      import_sources: None,
      treeshake_compensation: Some(true),
      enable_inlined_conditional_merge: Some(true),
      enable_media_query_order: Some(true),
      enable_logical_styles_polyfill: Some(false),
      enable_ltr_rtl_comments: Some(false),
      enable_legacy_value_flipping: Some(false),
      dev: Some(false),
      test: Some(false),
      debug: None,
      enable_debug_data_prop: Some(true),
      // Absent on purpose: the option follows `dev` when it is not set.
      enable_dev_class_names: None,
      enable_minified_keys: Some(true),
      inject_stylex_side_effects: Some(false),
      use_real_file_for_source: Some(true),
      aliases: None,
      unstable_module_resolution: None,
      sx_prop_name: None,
      max_evaluation_depth: None,
      max_folded_characters: None,
      max_folded_entries: None,
      env: None,
      debug_file_path: None,
    }
  }
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ModuleResolutionKind {
  Haste,
  #[serde(rename = "commonjs", alias = "commonJS")]
  CommonJs,
  CrossFileParsing,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all(deserialize = "camelCase", serialize = "PascalCase"))]
pub struct ModuleResolution {
  #[serde(rename = "type")]
  pub kind: ModuleResolutionKind,
  pub root_dir: Option<String>,
  pub theme_file_extension: Option<String>,
}

impl ModuleResolution {
  pub fn haste(root_dir: Option<String>) -> Self {
    ModuleResolution {
      kind: ModuleResolutionKind::Haste,
      root_dir,
      theme_file_extension: None,
    }
  }

  pub fn common_js(root_dir: Option<String>) -> Self {
    ModuleResolution {
      kind: ModuleResolutionKind::CommonJs,
      root_dir,
      theme_file_extension: None,
    }
  }

  pub fn cross_file_parsing(root_dir: Option<String>) -> Self {
    ModuleResolution {
      kind: ModuleResolutionKind::CrossFileParsing,
      root_dir,
      theme_file_extension: None,
    }
  }
}

#[derive(Deserialize, Debug, Clone)]
pub enum CheckModuleResolution {
  CommonJs {
    root_dir: Option<String>,
    theme_file_extension: Option<String>,
  },
  Haste {
    root_dir: Option<String>,
    theme_file_extension: Option<String>,
  },
  CrossFileParsing {
    root_dir: Option<String>,
    theme_file_extension: Option<String>,
  },
}

impl CheckModuleResolution {
  pub fn root_dir(&self) -> Option<&str> {
    match self {
      CheckModuleResolution::CommonJs { root_dir, .. }
      | CheckModuleResolution::Haste { root_dir, .. }
      | CheckModuleResolution::CrossFileParsing { root_dir, .. } => root_dir.as_deref(),
    }
  }

  pub fn theme_file_extension(&self) -> Option<&str> {
    match self {
      CheckModuleResolution::CommonJs {
        theme_file_extension,
        ..
      }
      | CheckModuleResolution::Haste {
        theme_file_extension,
        ..
      }
      | CheckModuleResolution::CrossFileParsing {
        theme_file_extension,
        ..
      } => theme_file_extension.as_deref(),
    }
  }
}

impl From<ModuleResolution> for CheckModuleResolution {
  fn from(module_resolution: ModuleResolution) -> Self {
    let ModuleResolution {
      kind,
      root_dir,
      theme_file_extension,
    } = module_resolution;

    match kind {
      ModuleResolutionKind::Haste => CheckModuleResolution::Haste {
        root_dir,
        theme_file_extension,
      },
      ModuleResolutionKind::CommonJs => CheckModuleResolution::CommonJs {
        root_dir,
        theme_file_extension,
      },
      ModuleResolutionKind::CrossFileParsing => CheckModuleResolution::CrossFileParsing {
        root_dir,
        theme_file_extension,
      },
    }
  }
}

impl Default for CheckModuleResolution {
  fn default() -> Self {
    CheckModuleResolution::CommonJs {
      root_dir: None,
      theme_file_extension: None,
    }
  }
}

#[derive(Clone, Debug)]
pub struct StyleXOptions {
  pub core: CoreStyleXOptions,
  pub runtime_injection: RuntimeInjection,
}

impl Deref for StyleXOptions {
  type Target = CoreStyleXOptions;
  fn deref(&self) -> &Self::Target {
    &self.core
  }
}

impl DerefMut for StyleXOptions {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.core
  }
}

impl StyleXOptions {
  pub fn with_class_name_prefix(mut self, prefix: impl Into<String>) -> Self {
    self.core.class_name_prefix = prefix.into();
    self
  }

  pub fn with_style_resolution(mut self, resolution: StyleResolution) -> Self {
    self.core.style_resolution = resolution;
    self
  }

  pub fn with_debug(mut self, debug: bool) -> Self {
    self.core.debug = debug;
    self
  }

  pub fn with_dev(mut self, dev: bool) -> Self {
    self.core.dev = dev;
    self
  }

  pub fn with_test(mut self, test: bool) -> Self {
    self.core.test = test;
    self
  }

  pub fn with_enable_debug_data_prop(mut self, enabled: bool) -> Self {
    self.core.enable_debug_data_prop = enabled;
    self
  }

  pub fn with_enable_dev_class_names(mut self, enabled: bool) -> Self {
    self.core.enable_dev_class_names = enabled;
    self
  }

  pub fn with_enable_font_size_px_to_rem(mut self, enabled: bool) -> Self {
    self.core.enable_font_size_px_to_rem = enabled;
    self
  }

  pub fn with_enable_logical_styles_polyfill(mut self, enabled: bool) -> Self {
    self.core.enable_logical_styles_polyfill = enabled;
    self
  }

  pub fn with_enable_minified_keys(mut self, enabled: bool) -> Self {
    self.core.enable_minified_keys = enabled;
    self
  }

  pub fn with_runtime_injection(mut self, injection: RuntimeInjection) -> Self {
    self.runtime_injection = injection;
    self
  }

  pub fn with_unstable_module_resolution(mut self, resolution: CheckModuleResolution) -> Self {
    self.core.unstable_module_resolution = resolution;
    self
  }
}

impl Default for StyleXOptions {
  fn default() -> Self {
    StyleXOptions {
      core: CoreStyleXOptions::default(),
      runtime_injection: RuntimeInjection::Boolean(false),
    }
  }
}

impl From<StyleXOptionsParams> for StyleXOptions {
  fn from(options: StyleXOptionsParams) -> Self {
    let module_resolution = options
      .unstable_module_resolution
      .unwrap_or_else(|| ModuleResolution::common_js(None));

    let unstable_module_resolution = CheckModuleResolution::from(module_resolution);

    let runtime_injection = match options.runtime_injection {
      Some(RuntimeInjection::Boolean(true)) => {
        RuntimeInjection::Regular(DEFAULT_INJECT_PATH.to_string())
      },
      Some(RuntimeInjection::Regular(path)) => RuntimeInjection::Regular(path),
      Some(RuntimeInjection::Named(named)) => RuntimeInjection::Named(named),
      Some(RuntimeInjection::Boolean(false)) | None => RuntimeInjection::Boolean(false),
    };

    let core = CoreStyleXOptions::default()
      .with_unstable_module_resolution(unstable_module_resolution)
      .with_import_sources(options.import_sources.unwrap_or_else(|| {
        vec![
          ImportSources::Regular("stylex".to_string()),
          ImportSources::Regular("@stylexjs/stylex".to_string()),
        ]
      }))
      .with_debug(options.debug.or(options.dev).unwrap_or(false))
      .with_aliases(options.aliases)
      .with_sx_prop_name(match options.sx_prop_name {
        None => Some(STYLEX_SX.to_string()),
        Some(SxPropNameParam::Disabled) => None,
        // A blank name names no prop. The raw markup path cannot match one --
        // a JSX attribute name is never empty -- so accepting it verbatim
        // would let the compiled path rewrite `{ "": value }`, which no author
        // wrote as an `sx` prop. The module-selection scan refuses a blank
        // name for the same reason, so this keeps the two sides answering
        // alike.
        Some(SxPropNameParam::Enabled(name)) if name.trim().is_empty() => None,
        Some(SxPropNameParam::Enabled(name)) => Some(name),
      })
      .with_env(options.env.unwrap_or_default())
      .with_debug_file_path(options.debug_file_path)
      .maybe_style_resolution(options.style_resolution)
      .maybe_property_validation_mode(options.property_validation_mode)
      .maybe_enable_font_size_px_to_rem(options.enable_font_size_px_to_rem)
      .maybe_class_name_prefix(options.class_name_prefix)
      .maybe_dev(options.dev)
      .maybe_test(options.test)
      .maybe_enable_debug_data_prop(options.enable_debug_data_prop)
      .maybe_enable_dev_class_names(options.enable_dev_class_names.or(options.dev))
      .maybe_enable_minified_keys(options.enable_minified_keys)
      .maybe_inject_stylex_side_effects(options.inject_stylex_side_effects)
      .maybe_treeshake_compensation(options.treeshake_compensation)
      .maybe_enable_inlined_conditional_merge(options.enable_inlined_conditional_merge)
      .maybe_enable_media_query_order(options.enable_media_query_order)
      .maybe_enable_logical_styles_polyfill(options.enable_logical_styles_polyfill)
      .maybe_enable_legacy_value_flipping(options.enable_legacy_value_flipping)
      .maybe_enable_ltr_rtl_comments(options.enable_ltr_rtl_comments)
      .maybe_use_real_file_for_source(options.use_real_file_for_source)
      .maybe_max_evaluation_depth(options.max_evaluation_depth)
      .maybe_max_folded_characters(options.max_folded_characters)
      .maybe_max_folded_entries(options.max_folded_entries);

    StyleXOptions {
      core,
      runtime_injection,
    }
  }
}

#[cfg(test)]
#[path = "tests/stylex_options_test.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/stylex_options_builders_test.rs"]
mod stylex_options_builders_test;
