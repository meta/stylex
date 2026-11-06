//! Tests for path utility functions: `contains_subpath` and `relative_path`.

use std::path::{Path, PathBuf};

use crate::utils::{contains_subpath, relative_path};

#[test]
fn contains_subpath_found() {
  assert!(contains_subpath(Path::new("a/b/c"), Path::new("b")));
}

#[test]
fn contains_subpath_not_found() {
  assert!(!contains_subpath(Path::new("a/b/c"), Path::new("d")));
}

#[test]
fn contains_subpath_first_segment() {
  assert!(contains_subpath(
    Path::new("node_modules/pkg/index.js"),
    Path::new("pkg")
  ));
}

#[test]
fn contains_subpath_exact_match() {
  assert!(contains_subpath(Path::new("a"), Path::new("a")));
}

#[test]
fn contains_subpath_empty_subpath() {
  assert!(!contains_subpath(Path::new("a/b/c"), Path::new("")));
}

#[test]
fn relative_path_basic() {
  #[cfg(unix)]
  {
    let root = PathBuf::from("/root");
    let file = root.join("src").join("file.ts");
    let result = relative_path(&file, &root);
    assert_eq!(result, PathBuf::from("src").join("file.ts"));
  }

  #[cfg(windows)]
  {
    let root = PathBuf::from(r"C:\root");
    let file = root.join("src").join("file.ts");
    let result = relative_path(&file, &root);
    assert_eq!(result, PathBuf::from("src").join("file.ts"));
  }
}

#[test]
fn relative_path_with_parent() {
  #[cfg(unix)]
  {
    let root = PathBuf::from("/root");
    let file = PathBuf::from("/other").join("file.ts");
    let result = relative_path(&file, &root);
    assert_eq!(result, PathBuf::from("..").join("other").join("file.ts"));
  }

  #[cfg(windows)]
  {
    let root = PathBuf::from(r"C:\root");
    let file = PathBuf::from(r"C:\other").join("file.ts");
    let result = relative_path(&file, &root);
    assert_eq!(result, PathBuf::from("..").join("other").join("file.ts"));
  }
}
