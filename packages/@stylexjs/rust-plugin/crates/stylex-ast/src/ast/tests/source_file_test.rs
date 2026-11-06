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

// extract_filename_from_path

mod extract_filename_from_path_tests {}

// extract_path

mod extract_path_tests {}

// extract_filename_with_ext_from_path

mod extract_filename_with_ext_from_path_tests {}

// Virtual files

/// Virtual files give back an empty value rather than an error.
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
  fn every_virtual_file_has_no_file_name() {
    for name in virtual_names() {
      assert_eq!(extract_filename_with_ext_from_path(&name), None, "{name:?}");
    }
  }
}

// Edge-case real paths

mod unusual_path_tests {}

// Paths with no name to read

/// Real paths naming no file panic rather than produce a wrong class name.
mod nameless_path_tests {
  use super::*;

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
}

// Paths that are not valid UTF-8

/// Non-UTF-8 names panic, since everything downstream is UTF-8. The invalid name is built per
/// platform so the tests run everywhere.
mod invalid_utf8_path_tests {
  use super::*;

  /// One unit no UTF-8 decoder accepts: `0xff` on Unix, an unpaired surrogate on Windows.
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
  fn file_name_of_an_invalid_utf8_name_panics() {
    let name = FileName::Real(invalid_utf8_path("/src/file", ".js"));
    let result =
      std::panic::catch_unwind(|| extract_filename_with_ext_from_path(&name).map(str::to_string));
    assert_panic_message(result, INVALID_UTF8);
  }

  /// Only the whole-path reader fails on an invalid directory name.
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
