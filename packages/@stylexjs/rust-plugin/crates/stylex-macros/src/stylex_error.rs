//! Structured error types and formatting for StyleX diagnostics.
//!
//! Provides `StyleXError` and helper functions/macros that produce clean,
//! branded `[StyleX] <message>` output on both stderr and NAPI boundaries.

use colored::Colorize;
use std::{borrow::Cow, fmt};
use stylex_constants::logger::STYLEX_LOG_PREFIX;

/// Structured error for all user-facing StyleX diagnostics.
///
/// `Display` produces:
/// ```text
/// [StyleX] key > path > message
///   --> file:line
/// [Stack trace]: source_location    (whenever source_location is set)
/// ```
#[derive(Debug, Clone)]
pub struct StyleXError {
  pub message: Cow<'static, str>,
  pub file: Option<Cow<'static, str>>,
  pub key_path: Option<Vec<String>>,
  pub line: Option<usize>,
  pub col: Option<usize>,
  pub source_location: Option<Cow<'static, str>>,
}

impl StyleXError {
  pub fn with_location(
    mut self,
    file: impl Into<Cow<'static, str>>,
    line: usize,
    col: usize,
  ) -> Self {
    self.file = Some(file.into());
    self.line = Some(line);
    self.col = Some(col);
    self
  }

  pub fn with_key_path(mut self, key_path: impl Into<Vec<String>>) -> Self {
    self.key_path = Some(key_path.into());
    self
  }

  pub fn with_source_location(mut self, source_location: impl Into<Cow<'static, str>>) -> Self {
    self.source_location = Some(source_location.into());
    self
  }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl fmt::Display for StyleXError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    // Colored [StyleX] prefix
    write!(f, "{} ", "[StyleX]".bright_blue().bold())?;

    // Key path breadcrumbs (if any)
    if let Some(ref keys) = self.key_path {
      for key in keys {
        write!(f, "{} > ", key.dimmed().cyan())?;
      }
    }

    // Main error message
    write!(f, "{}", self.message.as_ref().red())?;

    // File location (when available)
    if let Some(ref file) = self.file {
      let file = file.as_ref();
      match (self.line, self.col) {
        (Some(line), Some(col)) => {
          write!(f, "\n  --> {file}:{line}:{col}")?;
        },
        (Some(line), None) => {
          write!(f, "\n  --> {file}:{line}")?;
        },
        _ => {
          write!(f, "\n  --> {file}")?;
        },
      }
    }

    // Stack trace (printed whenever source_location is set)
    if let Some(ref src) = self.source_location
      && log::log_enabled!(log::Level::Info)
    {
      write!(
        f,
        "\n{}: {}",
        "[Stack trace]".dimmed().yellow(),
        src.as_ref()
      )?;
    }

    Ok(())
  }
}

impl std::error::Error for StyleXError {}

/// Returns `true` while a [`SuppressPanicStderr`] guard is alive on this
/// thread.
///
/// Used by the custom panic hook installed in `logger::initialize()` to avoid
/// printing anything when a panic is caught by `std::panic::catch_unwind`.
pub fn is_panic_stderr_suppressed() -> bool {
  SUPPRESS_PANIC_STDERR.with(|f| f.get())
}

/// RAII guard that suppresses panic-hook stderr output for its lifetime.
///
/// Create one immediately before `catch_unwind`; the hook will stay silent
/// until the guard is dropped (on exit from scope, including on panic).
///
/// ```rust,ignore
/// let _guard = SuppressPanicStderr::new();
/// let result = std::panic::catch_unwind(|| { /* ... */ });
/// // guard dropped here -> suppression lifted
/// ```
pub struct SuppressPanicStderr;

impl SuppressPanicStderr {
  pub fn new() -> Self {
    SUPPRESS_PANIC_STDERR.with(|f| f.set(true));
    Self
  }
}

impl Default for SuppressPanicStderr {
  fn default() -> Self {
    Self::new()
  }
}

impl Drop for SuppressPanicStderr {
  fn drop(&mut self) {
    SUPPRESS_PANIC_STDERR.with(|f| f.set(false));
  }
}

// ---------------------------------------------------------------------------
// Utilities for the NAPI boundary
// ---------------------------------------------------------------------------

/// Strip ANSI escape sequences from a string.
fn strip_ansi(s: &str) -> Cow<'_, str> {
  if !s.as_bytes().contains(&b'\x1B') {
    return Cow::Borrowed(s);
  }

  let mut result = String::with_capacity(s.len());
  let mut chars = s.chars().peekable();
  while let Some(ch) = chars.next() {
    if ch == '\x1B' && chars.peek() == Some(&'[') {
      chars.next(); // consume '['
      for c in chars.by_ref() {
        if c == 'm' {
          break;
        }
      }
    } else {
      result.push(ch);
    }
  }
  Cow::Owned(result)
}

/// Extract a plain-text error message from a caught panic payload.
///
/// ANSI codes are stripped before the prefix check so that colored
/// `StyleXError` payloads are detected correctly.  The returned string
/// is always plain text, safe to pass to the NAPI boundary or non-TTY logs.
///
/// If the stripped message contains `[StyleX]`, it is returned as-is.
/// Otherwise, it is wrapped as `[StyleX] <message>`.
pub fn format_panic_message(error: &Box<dyn std::any::Any + Send>) -> String {
  // How to get stack trace from the error?
  let raw = match error.downcast_ref::<String>() {
    Some(s) => s.as_str(),
    None => match error.downcast_ref::<&str>() {
      Some(s) => *s,
      None => {
        return format!("{} Unknown error during transformation", STYLEX_LOG_PREFIX);
      },
    },
  };

  let plain = strip_ansi(raw);

  if plain.contains(STYLEX_LOG_PREFIX) {
    plain.into_owned()
  } else {
    format!("{} {}", STYLEX_LOG_PREFIX, plain)
  }
}

// ---------------------------------------------------------------------------
// Colour output
// ---------------------------------------------------------------------------

/// Renders every diagnostic this module colours as plain text, process-wide.
///
/// `StyleXError`'s `Display` colours the `[StyleX]` prefix and the message, and
/// whether it does is decided at runtime by whether stderr is a terminal and by
/// `NO_COLOR` / `CLICOLOR_FORCE`. A caller that asserts on a rendered
/// diagnostic therefore reads a different string piped to a file than it does
/// in a terminal, unless it has called this first.
///
/// Lives beside the `Display` that does the colouring rather than at each
/// caller: the override belongs to whoever owns the colours.
pub fn disable_colour_output() {
  colored::control::set_override(false);
}

// ---------------------------------------------------------------------------
// Panic-output suppression (used around `catch_unwind` at the NAPI boundary)
// ---------------------------------------------------------------------------

thread_local! {
  static SUPPRESS_PANIC_STDERR: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
#[path = "tests/stylex_error_test.rs"]
mod tests;
