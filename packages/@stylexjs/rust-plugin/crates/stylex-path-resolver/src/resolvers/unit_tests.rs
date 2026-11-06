//! Unit tests for path resolver internal helpers: `file_not_found_error`
//! and `possible_aliased_paths`.

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

/// file_not_found_error should create a NotFound IO error containing the import
/// path.
#[test]
fn file_not_found_error_creates_not_found() {
  let err = file_not_found_error("./foo/bar");
  assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
  assert!(err.to_string().contains("./foo/bar"));
}

/// With empty aliases map, possible_aliased_paths should return only the
/// original path.
#[test]
fn possible_aliased_paths_empty_aliases() {
  let aliases = FxHashMap::default();
  let paths = possible_aliased_paths("@pkg/foo", &aliases);
  assert_eq!(paths, vec![PathBuf::from("@pkg/foo")]);
}

/// Exact alias match should add the aliased path after the original.
#[test]
fn possible_aliased_paths_exact_match() {
  let mut aliases = FxHashMap::default();
  aliases.insert("@pkg/foo".to_string(), vec!["/abs/path/foo".to_string()]);
  let paths = possible_aliased_paths("@pkg/foo", &aliases);
  assert_eq!(paths.len(), 2);
  assert_eq!(paths[0], PathBuf::from("@pkg/foo"));
  assert_eq!(paths[1], PathBuf::from("/abs/path/foo"));
}

/// Wildcard alias should expand the matched portion.
#[test]
fn possible_aliased_paths_wildcard_match() {
  let mut aliases = FxHashMap::default();
  aliases.insert("@src/*".to_string(), vec!["./src/*".to_string()]);
  let paths = possible_aliased_paths("@src/components/Button", &aliases);
  assert_eq!(paths.len(), 2);
  assert_eq!(paths[0], PathBuf::from("@src/components/Button"));
  assert_eq!(paths[1], PathBuf::from("./src/components/Button"));
}

/// Non-matching wildcard alias should return only the original path.
#[test]
fn possible_aliased_paths_no_match() {
  let mut aliases = FxHashMap::default();
  aliases.insert("@other/*".to_string(), vec!["./other/*".to_string()]);
  let paths = possible_aliased_paths("@pkg/foo", &aliases);
  assert_eq!(paths, vec![PathBuf::from("@pkg/foo")]);
}

/// Non-matching exact alias (no wildcard) should return only the original path.
/// Exercises the false branch of `else if alias == import_path_str`.
#[test]
fn possible_aliased_paths_exact_alias_no_match() {
  let mut aliases = FxHashMap::default();
  aliases.insert("@other/bar".to_string(), vec!["/abs/other".to_string()]);
  let paths = possible_aliased_paths("@pkg/foo", &aliases);
  assert_eq!(paths, vec![PathBuf::from("@pkg/foo")]);
}

/// Relative imports without extension should try known JS/TS extensions.
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

/// Missing relative import should return a NotFound error instead of panicking.
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

/// Source file path that has no parent should be rejected early.
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

/// If resolution fails from the source directory, resolver should retry from
/// root path.
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

/// Unknown bare package imports should return a NotFound error after all
/// fallbacks.
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

/// Aliases that expand to Turbopack's `/ROOT/` placeholder should be resolved
/// against the configured `root_dir`, mirroring how Turbopack rewrites project
/// paths. The `/ROOT/` prefix is replaced with `root_dir` and the remainder is
/// resolved with extensions.
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

/// Windows-style separators in alias values should still be recognized as the
/// Turbopack `/ROOT/` placeholder after normalization.
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

/// Relative aliases should resolve with normal module resolution from the
/// source file directory, not by probing paths relative to the process cwd.
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

/// Module-resolution aliases must ignore paths that cannot be represented as
/// UTF-8 import specifiers.
#[cfg(unix)]
#[test]
fn try_resolve_as_module_returns_none_for_non_utf8_path() {
  use std::{ffi::OsStr, os::unix::ffi::OsStrExt, path::Path};

  let invalid = Path::new(OsStr::from_bytes(b"\xff"));

  assert!(super::try_resolve_as_module(invalid, &fixture_root("application-pnpm")).is_none());
}

/// No-op `log` sink whose only purpose is to let `debug!` calls fire so their
/// (lazily-evaluated) arguments are exercised.
///
/// The message itself cannot be read back here: `src/tests.rs` installs
/// `pretty_env_logger` from a constructor before any case runs, and `log` takes
/// one logger per process, so a capturing logger can never be the one in place.
/// What a case controls is the level, and the level is what decides whether the
/// arguments run: the macro builds its `format_args!` -- and with it every
/// argument expression -- as soon as `log::max_level()` admits the level.
struct DebugSink;

impl log::Log for DebugSink {
  fn enabled(&self, _metadata: &log::Metadata) -> bool {
    true
  }
  fn log(&self, _record: &log::Record) {}
  fn flush(&self) {}
}

static DEBUG_SINK: DebugSink = DebugSink;

/// Turns debug logging on, and answers the level that was in force so the case
/// can put it back.
fn enable_debug_logging() -> log::LevelFilter {
  let previous_level = log::max_level();

  let _ = log::set_logger(&DEBUG_SINK);
  log::set_max_level(log::LevelFilter::Debug);

  previous_level
}

/// Same successful `/ROOT/` rewrite as above, but with debug logging enabled so
/// the `debug!("rewrote /ROOT/ aliased path …")` in `try_resolve_aliased_path`
/// actually formats its arguments. `debug!` is lazy — its argument expressions
/// only run when the active level admits `Debug` — so this is the path that
/// exercises the rewritten-path log line.
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

/// When an alias expands to a `/ROOT/` placeholder but no `root_dir` is
/// configured, the placeholder cannot be rewritten. The literal `/ROOT/...`
/// path does not exist on disk, so resolution falls through and ultimately
/// fails with a NotFound error.
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

/// The same unrewritable placeholder as above, with debug logging on, so the
/// `debug!("skipping /ROOT/ aliased path ...")` that explains the failure
/// formats its arguments. Without a logger the message is never built, and a
/// resolution that fails for this reason gives the reader nothing to go on.
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

/// When a `/ROOT/` alias is rewritten against a configured `root_dir` but the
/// target file does not exist under it, resolution must fail with NotFound. In
/// particular it must NOT silently fall back to probing the literal `/ROOT/...`
/// path on disk.
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

/// `resolve_path` must fall back to the original `processing_file` (rather
/// than the stripped path) when `processing_file` is **outside** of
/// `root_dir.parent().parent()` and `strip_prefix` therefore returns `Err`.
/// This exercises the `unwrap_or_else(|| processing_file.to_path_buf())`
/// closure in `resolve_path`; without this test the closure body is never
/// executed (the existing should_panic tests all live inside `root_dir`,
/// so they hit `.map(Path::to_path_buf)` instead).
#[test]
#[should_panic(expected = "Resolve path must be a file, but got: /tmp/")]
fn resolve_path_panics_when_strip_prefix_fails() {
  // root_dir.parent().parent() == "/foo", which is *not* a prefix of
  // `/tmp/...`, so `strip_prefix` returns `Err` and the unwrap_or_else
  // fallback fires.
  let root_dir = std::path::Path::new("/foo/bar/baz");
  let processing_file = std::path::Path::new("/tmp/not-a-supported.extension");

  let mut package_json_seen = FxHashMap::<String, PackageJsonExtended>::default();
  let _ = crate::resolvers::resolve_path(processing_file, root_dir, &mut package_json_seen);
}
