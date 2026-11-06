//! Tests for StyleXError formatting, SuppressPanicStderr guard,
//! ANSI stripping, and panic message extraction.

use crate::{
  panic_macros::stylex_err,
  stylex_error::{
    SuppressPanicStderr, disable_colour_output, format_panic_message, is_panic_stderr_suppressed,
  },
};

#[test]
fn stylex_error_builders_set_context_fields() {
  let err = stylex_err("missing property")
    .with_location("src/App.js", 12, 8)
    .with_key_path(vec!["styles".to_string(), "root".to_string()])
    .with_source_location("panic.rs:42");

  assert_eq!(err.message, "missing property");
  assert_eq!(err.file.as_deref(), Some("src/App.js"));
  assert_eq!(err.line, Some(12));
  assert_eq!(err.col, Some(8));
  assert_eq!(
    err.key_path.as_deref(),
    Some(["styles".to_string(), "root".to_string()].as_slice())
  );
  assert_eq!(err.source_location.as_deref(), Some("panic.rs:42"));
}

/// SuppressPanicStderr RAII guard should toggle the thread-local flag.
#[test]
fn suppress_panic_stderr_guard_toggles_flag() {
  assert!(!is_panic_stderr_suppressed());
  {
    let _guard = SuppressPanicStderr::new();
    assert!(is_panic_stderr_suppressed());
  }
  assert!(!is_panic_stderr_suppressed());
}

/// SuppressPanicStderr::default() should behave identically to ::new().
#[test]
fn suppress_panic_stderr_default_works() {
  assert!(!is_panic_stderr_suppressed());
  {
    #[allow(clippy::default_constructed_unit_structs)]
    let _guard = SuppressPanicStderr::default();
    assert!(is_panic_stderr_suppressed());
  }
  assert!(!is_panic_stderr_suppressed());
}

/// strip_ansi is private, but tested indirectly via format_panic_message.
/// Here we test the format_panic_message function which internally uses
/// strip_ansi. A String payload without [StyleX] prefix should be wrapped with
/// the prefix.
#[test]
fn format_panic_message_from_string_payload() {
  let msg = String::from("something went wrong");
  let payload: Box<dyn std::any::Any + Send> = Box::new(msg);
  let result = format_panic_message(&payload);
  assert_eq!(result, "[StyleX] something went wrong");
}

/// A &str payload should be converted and wrapped.
#[test]
fn format_panic_message_from_str_payload() {
  let msg: &str = "static error";
  let payload: Box<dyn std::any::Any + Send> = Box::new(msg);
  let result = format_panic_message(&payload);
  assert_eq!(result, "[StyleX] static error");
}

/// A payload already containing [StyleX] should be returned as-is (not
/// double-wrapped).
#[test]
fn format_panic_message_with_existing_prefix() {
  let msg = String::from("[StyleX] already prefixed");
  let payload: Box<dyn std::any::Any + Send> = Box::new(msg);
  let result = format_panic_message(&payload);
  assert_eq!(result, "[StyleX] already prefixed");
}

/// An unknown type (not String or &str) should produce a generic "Unknown
/// error" message.
#[test]
fn format_panic_message_unknown_type() {
  let payload: Box<dyn std::any::Any + Send> = Box::new(42i32);
  let result = format_panic_message(&payload);
  assert!(result.contains("[StyleX]"));
  assert!(result.contains("Unknown error"));
}

/// ANSI escape codes should be stripped before prefixing.
#[test]
fn format_panic_message_strips_ansi_codes() {
  let msg = String::from("\u{1b}[31mred error\u{1b}[0m");
  let payload: Box<dyn std::any::Any + Send> = Box::new(msg);
  let result = format_panic_message(&payload);
  assert_eq!(result, "[StyleX] red error");
}

/// `disable_colour_output` has to actually silence the `Display` colouring, not
/// merely be callable.
///
/// Asserted against a forced-on override rather than against ambient state, so
/// the test measures the override and not whether the runner happens to have a
/// terminal attached — which is the very ambiguity the function exists to
/// remove. Each nextest test is its own process, so the process-wide override
/// this sets cannot reach another test.
#[test]
fn disable_colour_output_renders_a_diagnostic_without_ansi_codes() {
  let err = stylex_err("something went wrong");

  colored::control::set_override(true);
  let coloured = err.to_string();
  assert!(
    coloured.contains('\u{1b}'),
    "expected the forced-on rendering to carry ANSI codes, got {:?}",
    coloured
  );

  disable_colour_output();

  let plain = err.to_string();
  assert!(
    !plain.contains('\u{1b}'),
    "expected no ANSI codes after disabling colour, got {:?}",
    plain
  );
  assert!(plain.contains("something went wrong"));
}
