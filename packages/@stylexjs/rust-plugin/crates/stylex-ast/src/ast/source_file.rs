//! Reading the parts of a source file's name that the compiler needs.
//!
//! A virtual file (anything other than a real path on disk) has no name to
//! read, so each of these answers with an empty value rather than refusing.

use stylex_constants::constants::messages::INVALID_UTF8;
use stylex_macros::stylex_panic;
use swc_core::common::FileName;

/// The file name without its extension, or an empty string for a virtual file.
pub fn extract_filename_from_path(path: &FileName) -> String {
  match path {
    FileName::Real(path_buf) => {
      let stem = match path_buf.file_stem() {
        Some(s) => s,
        None => stylex_panic!("File path has no file stem component."),
      };
      match stem.to_str() {
        Some(s) => s.to_string(),
        None => stylex_panic!("{}", INVALID_UTF8),
      }
    },
    _ => String::new(),
  }
}

/// The whole path, or an empty string for a virtual file.
pub fn extract_path(path: &FileName) -> &str {
  match path {
    FileName::Real(path_buf) => match path_buf.to_str() {
      Some(s) => s,
      None => stylex_panic!("{}", INVALID_UTF8),
    },
    _ => "",
  }
}

/// The file name with its extension, or `None` for a virtual file.
pub fn extract_filename_with_ext_from_path(path: &FileName) -> Option<&str> {
  match path {
    FileName::Real(path_buf) => {
      let name = match path_buf.file_name() {
        Some(n) => n,
        None => stylex_panic!("File path has no file name component."),
      };
      Some(match name.to_str() {
        Some(s) => s,
        None => stylex_panic!("{}", INVALID_UTF8),
      })
    },
    _ => None,
  }
}
