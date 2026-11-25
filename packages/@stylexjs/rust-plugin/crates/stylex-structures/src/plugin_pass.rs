use std::path::PathBuf;

use swc_core::common::FileName;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginPass {
  pub cwd: Option<PathBuf>,
  pub filename: FileName,
}

impl Default for PluginPass {
  fn default() -> Self {
    Self {
      cwd: None,
      filename: FileName::Anon,
    }
  }
}

impl PluginPass {
  /// Creates a `PluginPass`; missing paths default to fixed test-fixture values.
  pub fn new(cwd: Option<PathBuf>, filename: Option<FileName>) -> Self {
    PluginPass {
      cwd: cwd.or_else(|| Some(PathBuf::from("/stylex/packages/"))),
      filename: filename
        .unwrap_or_else(|| FileName::Real("/stylex/packages/TestTheme.stylex.js".into())),
    }
  }

  pub fn test_default() -> Self {
    Self::new(None, None)
  }

  pub fn with_cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
    self.cwd = Some(cwd.into());
    self
  }

  pub fn with_filename(mut self, filename: FileName) -> Self {
    self.filename = filename;
    self
  }
}

#[cfg(test)]
#[path = "tests/plugin_pass_test.rs"]
mod tests;
