//! Tests for filesystem path discovery helpers.

use std::{
  fs,
  path::{Path, PathBuf},
  time::{SystemTime, UNIX_EPOCH},
};

use super::find_closest_path;

fn temp_dir(prefix: &str) -> PathBuf {
  let nanos = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_nanos();
  let dir = std::env::temp_dir().join(format!("stylex-path-resolver-{prefix}-{nanos}"));
  fs::create_dir_all(&dir).unwrap();
  dir
}

fn cleanup(path: &Path) {
  let _ = fs::remove_dir_all(path);
}

/// Finds the nearest ancestor folder matching the requested name.
#[test]
fn find_closest_path_returns_nearest_match() {
  let root = temp_dir("find-closest-path-hit");
  let target = root.join("target-dir");
  let nested = root.join("a/b/c");

  fs::create_dir_all(&target).unwrap();
  fs::create_dir_all(&nested).unwrap();

  let found = find_closest_path(&nested, "target-dir");
  assert_eq!(found, Some(target));

  cleanup(&root);
}

/// Returns None after recursive search reaches filesystem root.
#[test]
fn find_closest_path_returns_none_when_target_missing() {
  let root = temp_dir("find-closest-path-miss");
  let nested = root.join("deep/inside/tree");
  fs::create_dir_all(&nested).unwrap();

  let found = find_closest_path(&nested, "missing-target-dir");
  assert_eq!(found, None);

  cleanup(&root);
}
