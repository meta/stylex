//! Tests for package-json and node_modules path discovery helpers.

use std::{
  fs,
  path::{Path, PathBuf},
  time::{SystemTime, UNIX_EPOCH},
};

use super::{
  find_closest_node_modules, find_closest_package_json, find_closest_package_json_folder,
  recursive_find_node_modules,
};

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

/// package.json lookup should resolve both file path and parent folder.
#[test]
fn finds_closest_package_json_and_folder() {
  let root = temp_dir("package-json-lookup");
  let nested = root.join("apps/web/src");
  let package_json = root.join("package.json");

  fs::create_dir_all(&nested).unwrap();
  fs::write(&package_json, "{}").unwrap();

  assert_eq!(find_closest_package_json(&nested), Some(package_json));
  assert_eq!(
    find_closest_package_json_folder(&nested),
    Some(root.clone())
  );

  cleanup(&root);
}

/// Missing node_modules should return an empty set.
#[test]
fn recursive_find_node_modules_returns_empty_when_absent() {
  let root = temp_dir("node-modules-empty");
  let nested = root.join("apps/web/src");
  fs::create_dir_all(&nested).unwrap();

  assert_eq!(find_closest_node_modules(&nested), None);
  assert!(recursive_find_node_modules(&nested, None).is_empty());

  cleanup(&root);
}

/// Explicit known `.git` should collect nested, intermediate, and
/// repository-root node_modules.
#[test]
fn recursive_find_node_modules_collects_known_git_tree() {
  let root = temp_dir("node-modules-known-git");
  let git_dir = root.join(".git");
  let root_modules = root.join("node_modules");
  let apps_modules = root.join("apps/node_modules");
  let web_modules = root.join("apps/web/node_modules");
  let source_dir = root.join("apps/web/src");

  fs::create_dir_all(&git_dir).unwrap();
  fs::create_dir_all(&root_modules).unwrap();
  fs::create_dir_all(&apps_modules).unwrap();
  fs::create_dir_all(&web_modules).unwrap();
  fs::create_dir_all(&source_dir).unwrap();

  let result = recursive_find_node_modules(&source_dir, Some(git_dir));

  assert_eq!(result.len(), 3);
  assert!(result.contains(&web_modules));
  assert!(result.contains(&apps_modules));
  assert!(result.contains(&root_modules));

  cleanup(&root);
}

/// When known_git_dir is omitted, resolver should discover `.git` in parent
/// directories.
#[test]
fn recursive_find_node_modules_discovers_git_from_path() {
  let root = temp_dir("node-modules-discover-git");
  let git_dir = root.join(".git");
  let root_modules = root.join("node_modules");
  let apps_modules = root.join("apps/node_modules");
  let web_modules = root.join("apps/web/node_modules");
  let source_dir = root.join("apps/web/src");

  fs::create_dir_all(&git_dir).unwrap();
  fs::create_dir_all(&root_modules).unwrap();
  fs::create_dir_all(&apps_modules).unwrap();
  fs::create_dir_all(&web_modules).unwrap();
  fs::create_dir_all(&source_dir).unwrap();

  let result = recursive_find_node_modules(&source_dir, None);

  assert_eq!(result.len(), 3);
  assert!(result.contains(&web_modules));
  assert!(result.contains(&apps_modules));
  assert!(result.contains(&root_modules));

  cleanup(&root);
}

/// Root-path input should be handled without panics.
#[test]
fn recursive_find_node_modules_handles_filesystem_root() {
  let result = recursive_find_node_modules(Path::new("/"), None);
  let root_node_modules = PathBuf::from("/node_modules");

  if root_node_modules.exists() {
    assert!(result.contains(&root_node_modules));
  } else {
    assert!(result.is_empty());
  }
}
