//! Tests for `PluginPass` constructors and builder helpers.

use std::path::PathBuf;

use crate::plugin_pass::PluginPass;
use swc_core::common::FileName;

/// `test_default` is a convenience alias for fixture-based construction.
#[test]
fn plugin_pass_test_default_uses_fixture_values() {
  let pass = PluginPass::test_default();
  assert_eq!(pass.cwd, Some(PathBuf::from("/stylex/packages/")));
  assert!(matches!(pass.filename, FileName::Real(_)));
}
