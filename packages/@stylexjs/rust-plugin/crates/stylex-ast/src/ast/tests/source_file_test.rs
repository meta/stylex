//! Tests for reading the parts of a source file's name.

use std::{any::Any, path::PathBuf};

use stylex_constants::constants::messages::INVALID_UTF8;
use swc_core::common::FileName;

use crate::ast::source_file::{
  extract_filename_from_path, extract_filename_with_ext_from_path, extract_path,
};

/// A real path on disk, which is the only kind that has a name to read.
fn real(path: &str) -> FileName {
  FileName::Real(PathBuf::from(path))
}

/// Fails unless the call panicked with the given message.
///
/// A bare "it panicked" assertion also passes when an unrelated panic happens
/// first, which would hide the branch the test is there to reach.
fn assert_panic_message<T>(result: Result<T, Box<dyn Any + Send>>, expected: &str) {
  let payload = match result {
    Ok(_) => panic!("expected a panic carrying {expected:?}, but the call returned"),
    Err(payload) => payload,
  };
  let message = match payload.downcast_ref::<String>() {
    Some(message) => message.clone(),
    None => panic!("panic payload was not a string"),
  };
  assert!(
    message.contains(expected),
    "panic message {message:?} does not contain {expected:?}"
  );
}

// ──────────────────────────────────────────────
// extract_filename_from_path
// ──────────────────────────────────────────────

mod extract_filename_from_path_tests {
  use super::*;

  #[test]
  fn returns_stem_for_simple_js_file() {
    let path = FileName::Real(PathBuf::from("/path/to/file.js"));
    assert_eq!(extract_filename_from_path(&path), "file");
  }

  #[test]
  fn returns_stem_for_dotted_extension() {
    let path = FileName::Real(PathBuf::from("/path/to/file.stylex.ts"));
    assert_eq!(extract_filename_from_path(&path), "file.stylex");
  }

  #[test]
  fn returns_stem_for_flat_path() {
    let path = FileName::Real(PathBuf::from("simple.js"));
    assert_eq!(extract_filename_from_path(&path), "simple");
  }

  #[test]
  fn returns_empty_string_for_anon() {
    let path = FileName::Anon;
    assert_eq!(extract_filename_from_path(&path), "");
  }

  #[test]
  fn returns_stem_for_tsx_file() {
    let path = FileName::Real(PathBuf::from("/src/components/Button.tsx"));
    assert_eq!(extract_filename_from_path(&path), "Button");
  }

  #[test]
  fn returns_stem_for_no_extension() {
    let path = FileName::Real(PathBuf::from("/path/to/Makefile"));
    assert_eq!(extract_filename_from_path(&path), "Makefile");
  }
}

// ──────────────────────────────────────────────
// extract_path
// ──────────────────────────────────────────────

mod extract_path_tests {
  use super::*;

  #[test]
  fn returns_full_path_for_real_file() {
    let path = FileName::Real(PathBuf::from("/path/to/file.js"));
    assert_eq!(extract_path(&path), "/path/to/file.js");
  }

  #[test]
  fn returns_empty_for_anon() {
    let path = FileName::Anon;
    assert_eq!(extract_path(&path), "");
  }

  #[test]
  fn returns_relative_path() {
    let path = FileName::Real(PathBuf::from("relative/file.ts"));
    assert_eq!(extract_path(&path), "relative/file.ts");
  }
}

// ──────────────────────────────────────────────
// extract_filename_with_ext_from_path
// ──────────────────────────────────────────────

mod extract_filename_with_ext_from_path_tests {
  use super::*;

  #[test]
  fn returns_filename_with_ext_for_js() {
    let path = FileName::Real(PathBuf::from("/path/to/file.js"));
    assert_eq!(extract_filename_with_ext_from_path(&path), Some("file.js"));
  }

  #[test]
  fn returns_filename_with_double_ext() {
    let path = FileName::Real(PathBuf::from("/path/to/file.stylex.ts"));
    assert_eq!(
      extract_filename_with_ext_from_path(&path),
      Some("file.stylex.ts")
    );
  }

  #[test]
  fn returns_none_for_anon() {
    let path = FileName::Anon;
    assert_eq!(extract_filename_with_ext_from_path(&path), None);
  }

  #[test]
  fn returns_filename_without_directory() {
    let path = FileName::Real(PathBuf::from("standalone.css"));
    assert_eq!(
      extract_filename_with_ext_from_path(&path),
      Some("standalone.css")
    );
  }

  #[test]
  fn returns_filename_without_extension() {
    let path = FileName::Real(PathBuf::from("/path/Makefile"));
    assert_eq!(extract_filename_with_ext_from_path(&path), Some("Makefile"));
  }
}

// ──────────────────────────────────────────────
// Virtual files
// ──────────────────────────────────────────────

/// A virtual file has no name to read. Each variant gives back an empty value
/// instead of an error, so the caller does not have to know which kind of
/// virtual file it holds.
mod virtual_file_tests {
  use super::*;

  fn virtual_names() -> Vec<FileName> {
    vec![
      FileName::Anon,
      FileName::Macros("stylex".to_string()),
      FileName::QuoteExpansion,
      FileName::MacroExpansion,
      FileName::ProcMacroSourceCode,
      FileName::Internal("generated".to_string()),
      FileName::Custom("<stdin>".to_string()),
      FileName::Custom(String::new()),
    ]
  }

  #[test]
  fn every_virtual_file_has_no_stem() {
    for name in virtual_names() {
      assert_eq!(extract_filename_from_path(&name), "", "{name:?}");
    }
  }

  #[test]
  fn every_virtual_file_has_no_path() {
    for name in virtual_names() {
      assert_eq!(extract_path(&name), "", "{name:?}");
    }
  }

  #[test]
  fn every_virtual_file_has_no_file_name() {
    for name in virtual_names() {
      assert_eq!(extract_filename_with_ext_from_path(&name), None, "{name:?}");
    }
  }
}

// ──────────────────────────────────────────────
// Real paths at the edge of what a path may be
// ──────────────────────────────────────────────

mod unusual_path_tests {
  use super::*;

  /// A dot-file has no extension to strip, so the whole name is the stem.
  #[test]
  fn reads_a_dot_file_as_its_own_stem() {
    assert_eq!(
      extract_filename_from_path(&real("/src/.babelrc")),
      ".babelrc"
    );
    assert_eq!(
      extract_filename_with_ext_from_path(&real("/src/.babelrc")),
      Some(".babelrc")
    );
  }

  /// A trailing slash names the directory. The last component is still read as
  /// the file name.
  #[test]
  fn reads_through_a_trailing_slash() {
    assert_eq!(
      extract_filename_from_path(&real("/src/components/")),
      "components"
    );
    assert_eq!(
      extract_filename_with_ext_from_path(&real("/src/components/")),
      Some("components")
    );
  }

  #[test]
  fn reads_names_that_are_not_plain_ascii() {
    for (path, stem, name) in [
      ("/src/Ünïcödé.js", "Ünïcödé", "Ünïcödé.js"),
      ("/src/🎨.tsx", "🎨", "🎨.tsx"),
      ("/src/日本語.stylex.ts", "日本語.stylex", "日本語.stylex.ts"),
      ("/src/with space.js", "with space", "with space.js"),
      ("/src/tab\tname.js", "tab\tname", "tab\tname.js"),
    ] {
      assert_eq!(extract_filename_from_path(&real(path)), stem, "{path}");
      assert_eq!(
        extract_filename_with_ext_from_path(&real(path)),
        Some(name),
        "{path}"
      );
      assert_eq!(extract_path(&real(path)), path, "{path}");
    }
  }

  /// A generated or bundled path can be much deeper than a path written by
  /// hand. The readers must give the same answer for it.
  #[test]
  fn reads_an_extremely_long_and_deep_path() {
    let deep = "/".to_string() + &"segment/".repeat(10_000) + "file.stylex.js";
    let path = real(&deep);

    assert_eq!(extract_filename_from_path(&path), "file.stylex");
    assert_eq!(
      extract_filename_with_ext_from_path(&path),
      Some("file.stylex.js")
    );
    assert_eq!(extract_path(&path), deep);
  }

  #[test]
  fn reads_a_single_component_with_no_directory() {
    assert_eq!(extract_filename_from_path(&real("file.js")), "file");
    assert_eq!(extract_path(&real("file.js")), "file.js");
  }
}

// ──────────────────────────────────────────────
// Paths with no name to read
// ──────────────────────────────────────────────

/// These real paths name no file. A compiled module cannot have such a path,
/// so the readers stop with a panic. An invented empty name would give a wrong
/// class name instead.
mod nameless_path_tests {
  use super::*;

  /// A root, a parent reference and an empty path have no last component.
  fn nameless_paths() -> Vec<&'static str> {
    vec!["/", "..", "", "/..", "../..", "./.."]
  }

  #[test]
  fn stem_of_a_nameless_path_panics() {
    for path in nameless_paths() {
      let name = real(path);
      let result = std::panic::catch_unwind(|| extract_filename_from_path(&name));
      assert_panic_message(result, "File path has no file stem component.");
    }
  }

  #[test]
  fn file_name_of_a_nameless_path_panics() {
    for path in nameless_paths() {
      let name = real(path);
      let result =
        std::panic::catch_unwind(|| extract_filename_with_ext_from_path(&name).map(str::to_string));
      assert_panic_message(result, "File path has no file name component.");
    }
  }

  /// A nameless path is still a path. Reading the whole path thus works, even
  /// though reading a name does not.
  #[test]
  fn whole_path_of_a_nameless_path_is_readable() {
    for path in nameless_paths() {
      assert_eq!(extract_path(&real(path)), path);
    }
  }
}

// ──────────────────────────────────────────────
// Paths that are not valid UTF-8
// ──────────────────────────────────────────────

/// A file system can hold a name that is not valid UTF-8. Everything that reads
/// the name later — class names, injected CSS, source maps — is UTF-8 text, so
/// the readers stop with a panic instead of passing on bytes that nothing can
/// encode.
///
/// The invalid name is built differently per platform, but the tests are the
/// same everywhere. A platform-gated test module would leave these branches
/// uncovered on the platform it skips.
mod invalid_utf8_path_tests {
  use super::*;

  /// A path that holds one unit no UTF-8 decoder accepts: the byte `0xff` on
  /// Unix, an unpaired surrogate on Windows.
  #[cfg(unix)]
  fn invalid_utf8_path(prefix: &str, suffix: &str) -> PathBuf {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};

    let mut bytes = prefix.as_bytes().to_vec();
    bytes.push(0xff);
    bytes.extend_from_slice(suffix.as_bytes());
    PathBuf::from(OsString::from_vec(bytes))
  }

  #[cfg(windows)]
  fn invalid_utf8_path(prefix: &str, suffix: &str) -> PathBuf {
    use std::{ffi::OsString, os::windows::ffi::OsStringExt};

    let mut units: Vec<u16> = prefix.encode_utf16().collect();
    units.push(0xd800);
    units.extend(suffix.encode_utf16());
    PathBuf::from(OsString::from_wide(&units))
  }

  #[test]
  fn stem_of_an_invalid_utf8_name_panics() {
    let name = FileName::Real(invalid_utf8_path("/src/", ".js"));
    let result = std::panic::catch_unwind(|| extract_filename_from_path(&name));
    assert_panic_message(result, INVALID_UTF8);
  }

  #[test]
  fn whole_path_of_an_invalid_utf8_name_panics() {
    let name = FileName::Real(invalid_utf8_path("/src/file", ".js"));
    let result = std::panic::catch_unwind(|| extract_path(&name).to_string());
    assert_panic_message(result, INVALID_UTF8);
  }

  #[test]
  fn file_name_of_an_invalid_utf8_name_panics() {
    let name = FileName::Real(invalid_utf8_path("/src/file", ".js"));
    let result =
      std::panic::catch_unwind(|| extract_filename_with_ext_from_path(&name).map(str::to_string));
    assert_panic_message(result, INVALID_UTF8);
  }

  /// Invalid bytes in a directory name do not touch the file name. Only the
  /// reader of the whole path fails.
  #[test]
  fn invalid_bytes_in_a_directory_leave_the_file_name_readable() {
    let path = FileName::Real(invalid_utf8_path("/src/", "/file.js"));

    assert_eq!(extract_filename_from_path(&path), "file");
    assert_eq!(extract_filename_with_ext_from_path(&path), Some("file.js"));
    assert_panic_message(
      std::panic::catch_unwind(|| extract_path(&path).to_string()),
      INVALID_UTF8,
    );
  }
}
