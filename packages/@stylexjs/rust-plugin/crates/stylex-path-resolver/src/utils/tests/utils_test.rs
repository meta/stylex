//! Tests for path utility functions: `contains_subpath` and `relative_path`.

use std::path::Path;

use crate::utils::contains_subpath;

#[test]
fn contains_subpath_empty_subpath() {
  assert!(!contains_subpath(Path::new("a/b/c"), Path::new("")));
}
