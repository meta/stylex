//! Unit tests for the path resolver's internal helpers and its Windows path handling.

use std::{
  fs,
  path::PathBuf,
  time::{SystemTime, UNIX_EPOCH},
};

use rustc_hash::FxHashMap;

use crate::{
  package_json::PackageJsonExtended,
  resolvers::{file_not_found_error, possible_aliased_paths, resolve_file_path},
};

fn fixture_root(name: &str) -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("fixtures")
    .join(name)
}

fn temp_dir(prefix: &str) -> PathBuf {
  let nanos = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_nanos();
  let dir = std::env::temp_dir().join(format!("stylex-path-resolver-{prefix}-{nanos}"));
  fs::create_dir_all(&dir).unwrap();
  dir
}

#[test]
fn file_not_found_error_creates_not_found() {
  let err = file_not_found_error("./foo/bar");
  assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
  assert!(err.to_string().contains("./foo/bar"));
}

#[test]
fn possible_aliased_paths_empty_aliases() {
  let aliases = FxHashMap::default();
  let paths = possible_aliased_paths("@pkg/foo", &aliases);
  assert_eq!(paths, vec![PathBuf::from("@pkg/foo")]);
}

#[test]
fn possible_aliased_paths_exact_match() {
  let mut aliases = FxHashMap::default();
  aliases.insert("@pkg/foo".to_string(), vec!["/abs/path/foo".to_string()]);
  let paths = possible_aliased_paths("@pkg/foo", &aliases);
  assert_eq!(paths.len(), 2);
  assert_eq!(paths[0], PathBuf::from("@pkg/foo"));
  assert_eq!(paths[1], PathBuf::from("/abs/path/foo"));
}

#[test]
fn possible_aliased_paths_wildcard_match() {
  let mut aliases = FxHashMap::default();
  aliases.insert("@src/*".to_string(), vec!["./src/*".to_string()]);
  let paths = possible_aliased_paths("@src/components/Button", &aliases);
  assert_eq!(paths.len(), 2);
  assert_eq!(paths[0], PathBuf::from("@src/components/Button"));
  assert_eq!(paths[1], PathBuf::from("./src/components/Button"));
}

#[test]
fn possible_aliased_paths_no_match() {
  let mut aliases = FxHashMap::default();
  aliases.insert("@other/*".to_string(), vec!["./other/*".to_string()]);
  let paths = possible_aliased_paths("@pkg/foo", &aliases);
  assert_eq!(paths, vec![PathBuf::from("@pkg/foo")]);
}

#[test]
fn possible_aliased_paths_exact_alias_no_match() {
  let mut aliases = FxHashMap::default();
  aliases.insert("@other/bar".to_string(), vec!["/abs/other".to_string()]);
  let paths = possible_aliased_paths("@pkg/foo", &aliases);
  assert_eq!(paths, vec![PathBuf::from("@pkg/foo")]);
}

#[test]
fn resolve_file_path_resolves_relative_import_without_extension() {
  let root = fixture_root("application-pnpm");
  let source_file = root.join("src/pages/home.js");

  let mut package_json_seen = FxHashMap::<String, PackageJsonExtended>::default();
  let aliases = FxHashMap::<String, Vec<String>>::default();

  let resolved = resolve_file_path(
    "../components/button",
    source_file.to_str().unwrap(),
    root.to_str().unwrap(),
    &aliases,
    None,
    &mut package_json_seen,
  )
  .unwrap();

  assert_eq!(resolved, root.join("src/components/button.js"));
}

#[test]
fn resolve_file_path_returns_not_found_for_missing_relative_import() {
  let root = fixture_root("application-pnpm");
  let source_file = root.join("src/pages/home.js");

  let mut package_json_seen = FxHashMap::<String, PackageJsonExtended>::default();
  let aliases = FxHashMap::<String, Vec<String>>::default();

  let err = resolve_file_path(
    "../components/does-not-exist",
    source_file.to_str().unwrap(),
    root.to_str().unwrap(),
    &aliases,
    None,
    &mut package_json_seen,
  )
  .unwrap_err();

  assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
  assert!(err.to_string().contains("../components/does-not-exist"));
}

#[test]
fn resolve_file_path_rejects_source_path_without_parent() {
  let root = fixture_root("application-pnpm");
  let mut package_json_seen = FxHashMap::<String, PackageJsonExtended>::default();
  let aliases = FxHashMap::<String, Vec<String>>::default();

  let err = resolve_file_path(
    "stylex-lib-dist-main",
    "/",
    root.to_str().unwrap(),
    &aliases,
    None,
    &mut package_json_seen,
  )
  .unwrap_err();

  assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
fn resolve_file_path_falls_back_to_root_resolver() {
  let root = fixture_root("application-pnpm");
  let outside_dir = temp_dir("resolve-from-root-fallback");
  let source_file = outside_dir.join("entry.js");

  fs::write(&source_file, "export {};").unwrap();

  let mut package_json_seen = FxHashMap::<String, PackageJsonExtended>::default();
  let aliases = FxHashMap::<String, Vec<String>>::default();

  let resolved = resolve_file_path(
    "stylex-lib-dist-main",
    source_file.to_str().unwrap(),
    root.to_str().unwrap(),
    &aliases,
    None,
    &mut package_json_seen,
  )
  .unwrap();

  assert_eq!(
    resolved,
    root.join("node_modules/stylex-lib-dist-main/dist/index.jsx")
  );

  fs::remove_dir_all(outside_dir).unwrap();
}

#[test]
fn resolve_file_path_returns_not_found_for_unknown_package() {
  let root = fixture_root("application-pnpm");
  let source_file = root.join("src/pages/home.js");

  let mut package_json_seen = FxHashMap::<String, PackageJsonExtended>::default();
  let aliases = FxHashMap::<String, Vec<String>>::default();

  let err = resolve_file_path(
    "stylex-lib-package-that-does-not-exist",
    source_file.to_str().unwrap(),
    root.to_str().unwrap(),
    &aliases,
    None,
    &mut package_json_seen,
  )
  .unwrap_err();

  assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
  assert!(
    err
      .to_string()
      .contains("stylex-lib-package-that-does-not-exist")
  );
}

/// The `/ROOT/` placeholder is replaced with `root_dir`, as Turbopack rewrites project paths.
#[test]
fn resolve_file_path_resolves_root_placeholder_alias() {
  let root = fixture_root("application-pnpm");

  let mut package_json_seen = FxHashMap::<String, PackageJsonExtended>::default();
  let mut aliases = FxHashMap::<String, Vec<String>>::default();
  aliases.insert("@consts/*".to_string(), vec!["/ROOT/src/*".to_string()]);

  let resolved = resolve_file_path(
    "@consts/colors.stylex",
    root.join("src/pages/home.js").to_str().unwrap(),
    root.to_str().unwrap(),
    &aliases,
    root.to_str(),
    &mut package_json_seen,
  )
  .unwrap();

  assert_eq!(resolved, root.join("src/colors.stylex.js"));
}

#[test]
fn resolve_file_path_resolves_backslash_root_placeholder_alias() {
  let root = fixture_root("application-pnpm");

  let mut package_json_seen = FxHashMap::<String, PackageJsonExtended>::default();
  let mut aliases = FxHashMap::<String, Vec<String>>::default();
  aliases.insert("@consts/*".to_string(), vec!["\\ROOT\\src\\*".to_string()]);

  let resolved = resolve_file_path(
    "@consts/colors.stylex",
    root.join("src/pages/home.js").to_str().unwrap(),
    root.to_str().unwrap(),
    &aliases,
    root.to_str(),
    &mut package_json_seen,
  )
  .unwrap();

  assert_eq!(resolved, root.join("src/colors.stylex.js"));
}

/// Relative aliases go through module resolution from the source directory, not the process cwd.
#[test]
fn resolve_file_path_resolves_relative_alias_via_module_resolver() {
  let root = fixture_root("application-pnpm");

  let mut package_json_seen = FxHashMap::<String, PackageJsonExtended>::default();
  let mut aliases = FxHashMap::<String, Vec<String>>::default();
  aliases.insert("@consts/*".to_string(), vec!["../*".to_string()]);

  let resolved = resolve_file_path(
    "@consts/colors.stylex",
    root.join("src/pages/home.js").to_str().unwrap(),
    root.to_str().unwrap(),
    &aliases,
    None,
    &mut package_json_seen,
  )
  .unwrap();

  assert_eq!(resolved, root.join("src/colors.stylex.js"));
}

#[cfg(unix)]
#[test]
fn try_resolve_as_module_returns_none_for_non_utf8_path() {
  use std::{ffi::OsStr, os::unix::ffi::OsStrExt, path::Path};

  let invalid = Path::new(OsStr::from_bytes(b"\xff"));

  assert!(super::try_resolve_as_module(invalid, &fixture_root("application-pnpm")).is_none());
}

/// A no-op logger that lets `debug!` calls evaluate their lazy arguments.
/// `src/tests.rs` already installs `pretty_env_logger`, so only the level can be controlled here.
struct DebugSink;

impl log::Log for DebugSink {
  fn enabled(&self, _metadata: &log::Metadata) -> bool {
    true
  }
  fn log(&self, _record: &log::Record) {}
  fn flush(&self) {}
}

static DEBUG_SINK: DebugSink = DebugSink;

/// Turns debug logging on and returns the previous level so the case can restore it.
fn enable_debug_logging() -> log::LevelFilter {
  let previous_level = log::max_level();

  let _ = log::set_logger(&DEBUG_SINK);
  log::set_max_level(log::LevelFilter::Debug);

  previous_level
}

/// Debug logging is enabled so the lazy `debug!` arguments for the rewritten path are evaluated.
#[test]
#[serial_test::serial]
fn resolve_file_path_root_placeholder_rewrite_logs_at_debug() {
  let previous_level = enable_debug_logging();

  let root = fixture_root("application-pnpm");

  let mut package_json_seen = FxHashMap::<String, PackageJsonExtended>::default();
  let mut aliases = FxHashMap::<String, Vec<String>>::default();
  aliases.insert("@consts/*".to_string(), vec!["/ROOT/src/*".to_string()]);

  let resolved = resolve_file_path(
    "@consts/colors.stylex",
    root.join("src/pages/home.js").to_str().unwrap(),
    root.to_str().unwrap(),
    &aliases,
    root.to_str(),
    &mut package_json_seen,
  )
  .unwrap();

  assert_eq!(resolved, root.join("src/colors.stylex.js"));
  log::set_max_level(previous_level);
}

#[test]
fn resolve_file_path_root_placeholder_without_root_dir_is_not_found() {
  let root = fixture_root("application-pnpm");

  let mut package_json_seen = FxHashMap::<String, PackageJsonExtended>::default();
  let mut aliases = FxHashMap::<String, Vec<String>>::default();
  aliases.insert("@consts/*".to_string(), vec!["/ROOT/src/*".to_string()]);

  let err = resolve_file_path(
    "@consts/colors.stylex",
    root.join("src/pages/home.js").to_str().unwrap(),
    root.to_str().unwrap(),
    &aliases,
    None,
    &mut package_json_seen,
  )
  .unwrap_err();

  assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
  assert!(err.to_string().contains("@consts/colors.stylex"));
}

/// Debug logging is enabled so the `debug!` explaining the skipped placeholder is evaluated.
#[test]
#[serial_test::serial]
fn resolve_file_path_root_placeholder_without_root_dir_says_why_at_debug() {
  let previous_level = enable_debug_logging();

  let root = fixture_root("application-pnpm");

  let mut package_json_seen = FxHashMap::<String, PackageJsonExtended>::default();
  let mut aliases = FxHashMap::<String, Vec<String>>::default();
  aliases.insert("@consts/*".to_string(), vec!["/ROOT/src/*".to_string()]);

  let err = resolve_file_path(
    "@consts/colors.stylex",
    root.join("src/pages/home.js").to_str().unwrap(),
    root.to_str().unwrap(),
    &aliases,
    None,
    &mut package_json_seen,
  )
  .unwrap_err();

  log::set_max_level(previous_level);

  assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
}

/// The resolver must not fall back to probing the literal `/ROOT/...` path on disk.
#[test]
fn resolve_file_path_root_placeholder_missing_target_is_not_found() {
  let root = fixture_root("application-pnpm");

  let mut package_json_seen = FxHashMap::<String, PackageJsonExtended>::default();
  let mut aliases = FxHashMap::<String, Vec<String>>::default();
  aliases.insert("@consts/*".to_string(), vec!["/ROOT/src/*".to_string()]);

  let err = resolve_file_path(
    "@consts/does-not-exist.stylex",
    root.join("src/pages/home.js").to_str().unwrap(),
    root.to_str().unwrap(),
    &aliases,
    root.to_str(),
    &mut package_json_seen,
  )
  .unwrap_err();

  assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
  assert!(err.to_string().contains("@consts/does-not-exist.stylex"));
}

/// Covers the `unwrap_or_else` fallback in `resolve_path` when `strip_prefix` fails.
#[test]
#[should_panic(expected = "Resolve path must be a file, but got: /tmp/")]
fn resolve_path_panics_when_strip_prefix_fails() {
  // `/foo` is not a prefix of `/tmp/...`, so `strip_prefix` fails.
  let root_dir = std::path::Path::new("/foo/bar/baz");
  let processing_file = std::path::Path::new("/tmp/not-a-supported.extension");

  let mut package_json_seen = FxHashMap::<String, PackageJsonExtended>::default();
  let _ = crate::resolvers::resolve_path(processing_file, root_dir, &mut package_json_seen);
}

// ==================== a Windows path, read on every platform ====================

/// The import specifier must use `/`, which is what a package's `exports` field matches.
#[test]
fn a_windows_package_path_builds_a_slash_separated_import_specifier() {
  use crate::resolvers::{forward_slashes_on, package_import_specifiers, package_subpath};

  let relative = forward_slashes_on(
    r"..\packages\stylex-lib-dist-exports-local\colors.stylex.js".into(),
    true,
  );
  let subpath = package_subpath(&relative, "stylex-lib-dist-exports-local");

  assert_eq!(subpath, Some("/colors.stylex.js"));
  assert_eq!(
    package_import_specifiers("stylex-lib-dist-exports-local", subpath.unwrap()),
    vec![
      "stylex-lib-dist-exports-local/colors.stylex.js",
      "stylex-lib-dist-exports-local/colors.stylex",
    ]
  );

  let scoped = forward_slashes_on(
    r"..\..\packages\@stylex\theme-lib-main-dist\colors.stylex.js".into(),
    true,
  );
  assert_eq!(
    package_import_specifiers(
      "@stylex/theme-lib-main-dist",
      package_subpath(&scoped, "theme-lib-main-dist").unwrap()
    )[0],
    "@stylex/theme-lib-main-dist/colors.stylex.js"
  );

  // A path naming the package directory itself still reaches into it.
  let package_root = forward_slashes_on(r"..\packages\stylex-lib-dist-main".into(), true);
  assert_eq!(
    package_subpath(&package_root, "stylex-lib-dist-main"),
    Some("")
  );
}

#[test]
fn a_windows_resolved_path_reads_its_node_modules_package() {
  use crate::resolvers::{forward_slashes_on, node_modules_package};

  let scoped = forward_slashes_on(
    r"D:\a\app\node_modules\@stylex\lib-exports-pnpm\dist\colors.stylex.js".into(),
    true,
  );
  assert_eq!(
    node_modules_package(&scoped),
    Some((
      "D:/a/app/node_modules",
      "@stylex/lib-exports-pnpm".to_string(),
      "dist/colors.stylex.js"
    ))
  );

  let plain = forward_slashes_on(
    r"D:\a\app\node_modules\stylex-lib-pnpm\dist\index.jsx".into(),
    true,
  );
  assert_eq!(
    node_modules_package(&plain),
    Some((
      "D:/a/app/node_modules",
      "stylex-lib-pnpm".to_string(),
      "dist/index.jsx"
    ))
  );

  // A path already in the pnpm store is left as it is.
  let stored = forward_slashes_on(
    r"D:\a\app\node_modules\.pnpm\stylex-lib-pnpm@0.1.0\node_modules\stylex-lib-pnpm\dist\index.jsx"
      .into(),
    true,
  );
  assert_eq!(node_modules_package(&stored), None);
}

#[test]
fn a_backslash_is_a_file_name_character_off_windows() {
  use crate::resolvers::{forward_slashes_on, node_modules_package, package_subpath};

  let path = r"node_modules\pkg\file.js".to_string();

  assert_eq!(forward_slashes_on(path.clone(), false), path);
  assert_eq!(
    node_modules_package(&forward_slashes_on(path.clone(), false)),
    None
  );
  assert_eq!(
    package_subpath(&forward_slashes_on(path, false), "pkg"),
    Some(r"\file.js")
  );
}
