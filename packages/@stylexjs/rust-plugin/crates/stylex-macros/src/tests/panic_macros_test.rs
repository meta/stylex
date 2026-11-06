//! Tests for the `StyleXError` constructors and `unwrap_or_panic!`.

use crate::panic_macros::{stylex_err, stylex_err_with_file};

#[test]
fn stylex_err_creates_error_with_defaults() {
  let err = stylex_err("test message");
  assert_eq!(err.message, "test message");
  assert!(err.file.is_none());
  assert!(err.key_path.is_none());
  assert!(err.line.is_none());
  assert!(err.col.is_none());
  assert!(err.source_location.is_none());
}

#[test]
fn stylex_err_accepts_string_owned() {
  let err = stylex_err(String::from("owned message"));
  assert_eq!(err.message, "owned message");
}

#[test]
fn stylex_err_with_file_sets_file_field() {
  let err = stylex_err_with_file("msg", "src/main.rs");
  assert_eq!(err.message, "msg");
  assert_eq!(err.file.as_deref(), Some("src/main.rs"));
  assert!(err.key_path.is_none());
  assert!(err.line.is_none());
  assert!(err.col.is_none());
  assert!(err.source_location.is_none());
}

#[test]
fn stylex_err_with_file_accepts_owned_strings() {
  let err = stylex_err_with_file(String::from("msg"), String::from("file.rs"));
  assert_eq!(err.message, "msg");
  assert_eq!(err.file.as_deref(), Some("file.rs"));
}

#[test]
fn unwrap_or_panic_without_context_panics() {
  let result = std::panic::catch_unwind(|| {
    let failure: Result<(), &str> = Err("boom");
    crate::unwrap_or_panic!(failure);
  });

  assert!(result.is_err());
}
