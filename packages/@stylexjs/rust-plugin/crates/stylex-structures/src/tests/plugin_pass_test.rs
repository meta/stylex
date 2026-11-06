//! Tests for `PluginPass` constructors and builder helpers.

use std::path::PathBuf;

use crate::plugin_pass::PluginPass;
use swc_core::common::FileName;

#[test]
fn plugin_pass_default_values() {
  let pass = PluginPass::default();
  assert!(pass.cwd.is_none());
  assert!(matches!(pass.filename, FileName::Anon));
}

#[test]
fn plugin_pass_new_applies_fixture_fallbacks() {
  let pass = PluginPass::new(None, None);
  assert_eq!(pass.cwd, Some(PathBuf::from("/stylex/packages/")));
  assert!(matches!(pass.filename, FileName::Real(_)));
}

#[test]
fn plugin_pass_builder_methods_override_fields() {
  let pass = PluginPass::default()
    .with_cwd("/tmp/project")
    .with_filename(FileName::Real("/tmp/project/input.js".into()));

  assert_eq!(pass.cwd, Some(PathBuf::from("/tmp/project")));
  assert!(matches!(pass.filename, FileName::Real(_)));
}

/// `test_default` is a convenience alias for fixture-based construction.
#[test]
fn plugin_pass_test_default_uses_fixture_values() {
  let pass = PluginPass::test_default();
  assert_eq!(pass.cwd, Some(PathBuf::from("/stylex/packages/")));
  assert!(matches!(pass.filename, FileName::Real(_)));
}
