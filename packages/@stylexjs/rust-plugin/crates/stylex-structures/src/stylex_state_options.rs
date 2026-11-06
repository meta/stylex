use std::ops::{Deref, DerefMut};

use serde::Deserialize;

use stylex_constants::constants::common::DEFAULT_INJECT_PATH;
use stylex_enums::style_resolution::StyleResolution;

use crate::{
  core_stylex_options::CoreStyleXOptions,
  named_import_source::{RuntimeInjection, RuntimeInjectionState},
  stylex_options::{CheckModuleResolution, StyleXOptions},
};

#[derive(Deserialize, Clone, Debug, Default)]
pub struct StyleXStateOptions {
  #[serde(flatten)]
  pub core: CoreStyleXOptions,
  pub runtime_injection: Option<RuntimeInjectionState>,
}

impl Deref for StyleXStateOptions {
  type Target = CoreStyleXOptions;
  fn deref(&self) -> &Self::Target {
    &self.core
  }
}

impl DerefMut for StyleXStateOptions {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.core
  }
}

impl StyleXStateOptions {
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

  pub fn with_unstable_module_resolution(mut self, resolution: CheckModuleResolution) -> Self {
    self.core.unstable_module_resolution = resolution;
    self
  }

  pub fn with_runtime_injection(mut self, injection: Option<RuntimeInjectionState>) -> Self {
    self.runtime_injection = injection;
    self
  }
}

impl From<StyleXOptions> for StyleXStateOptions {
  fn from(options: StyleXOptions) -> Self {
    let runtime_injection = match options.runtime_injection {
      RuntimeInjection::Boolean(b) => {
        if b {
          Some(RuntimeInjectionState::Regular(
            DEFAULT_INJECT_PATH.to_string(),
          ))
        } else {
          None
        }
      },
      RuntimeInjection::Named(n) => Some(RuntimeInjectionState::Named(n)),
      RuntimeInjection::Regular(s) => Some(RuntimeInjectionState::Regular(s)),
    };

    StyleXStateOptions {
      core: options.core,
      runtime_injection,
    }
  }
}

thread_local! {
  /// The default options, built once for the thread.
  ///
  /// Read wherever a value must resolve the same way however the module is
  /// compiled: the hash a keyframes name is taken from, the text a position-try
  /// name is taken from, and the name of the default marker. Each of those built
  /// a set of its own for every call, and a set carries two owned strings and an
  /// empty shared env object.
  ///
  /// Thread-local rather than static, because the options hold an `Rc` and so
  /// cannot be shared between threads.
  static DEFAULT_OPTIONS: StyleXStateOptions = StyleXStateOptions::default();
}

/// Reads the shared default options.
///
/// A closure rather than a handed-back value, because the value stays owned by
/// the thread that holds it.
pub fn with_default_options<R>(read: impl FnOnce(&StyleXStateOptions) -> R) -> R {
  DEFAULT_OPTIONS.with(read)
}

#[cfg(test)]
#[path = "tests/stylex_state_options_test.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/stylex_state_options_builders_test.rs"]
mod stylex_state_options_builders_test;
