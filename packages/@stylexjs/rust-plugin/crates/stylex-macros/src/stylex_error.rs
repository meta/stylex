//! The `StyleXError` type and the helpers that report it on stderr and across the NAPI boundary.

use colored::Colorize;
use std::{borrow::Cow, fmt};
use stylex_constants::logger::STYLEX_LOG_PREFIX;

/// A user-facing StyleX diagnostic, displayed as `[StyleX] key > path > message` and its location.
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
    write!(f, "{} ", "[StyleX]".bright_blue().bold())?;

    if let Some(ref keys) = self.key_path {
      for key in keys {
        write!(f, "{} > ", key.dimmed().cyan())?;
      }
    }

    write!(f, "{}", self.message.as_ref().red())?;

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

/// Whether a [`SuppressPanicStderr`] guard is alive on this thread.
pub fn is_panic_stderr_suppressed() -> bool {
  SUPPRESS_PANIC_STDERR.with(|f| f.get())
}

/// An RAII guard that silences panic-hook stderr output while it is alive.
/// Guards nest: dropping one restores the state it found.
pub struct SuppressPanicStderr {
  /// The suppression in force before this guard, restored when it drops.
  previous: bool,
}

impl SuppressPanicStderr {
  pub fn new() -> Self {
    Self {
      previous: SUPPRESS_PANIC_STDERR.with(|f| f.replace(true)),
    }
  }
}

impl Default for SuppressPanicStderr {
  fn default() -> Self {
    Self::new()
  }
}

impl Drop for SuppressPanicStderr {
  fn drop(&mut self) {
    SUPPRESS_PANIC_STDERR.with(|f| f.set(self.previous));
  }
}

/// Strip ANSI escape sequences from a string.
fn strip_ansi(s: &str) -> Cow<'_, str> {
  if !s.as_bytes().contains(&b'\x1B') {
    return Cow::Borrowed(s);
  }

  let mut result = String::with_capacity(s.len());
  let mut chars = s.chars().peekable();
  while let Some(ch) = chars.next() {
    if ch == '\x1B' && chars.peek() == Some(&'[') {
      chars.next();
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

thread_local! {
  /// The byte span of the node the last StyleX error on this thread was reported at.
  static PANIC_SPAN: std::cell::Cell<Option<(u32, u32)>> = const { std::cell::Cell::new(None) };
  /// Whether the Babel plugin throws the last StyleX error as a plain `Error`, not a `SyntaxError`.
  static PANIC_PLAIN: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
  /// The message of the last StyleX error on this thread, without the key path or location.
  static PANIC_MESSAGE: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
  /// The binding whose declaration the last StyleX error is about, as Babel frames `binding.path`.
  static PANIC_DECLARATION: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// Reports the error at the node spanning `lo..hi`, overriding the fault's span and declaration.
pub fn report_panic_at(lo: u32, hi: u32) {
  PANIC_DECLARATION.with(|slot| slot.borrow_mut().take());
  record_panic_span(lo, hi);
}

/// Records that the StyleX error about to be raised is about the declaration of `name`.
pub fn record_panic_declaration(name: &str) {
  PANIC_DECLARATION.with(|slot| *slot.borrow_mut() = Some(name.to_owned()));
}

/// The declaration recorded for the last StyleX error on this thread, cleared.
pub fn take_panic_declaration() -> Option<String> {
  PANIC_DECLARATION.with(|slot| slot.borrow_mut().take())
}

/// Records the message of the StyleX error about to be raised.
pub fn record_panic_message(message: &str) {
  PANIC_MESSAGE.with(|slot| *slot.borrow_mut() = Some(message.to_owned()));
}

/// The message recorded for the last StyleX error on this thread, cleared.
pub fn take_panic_message() -> Option<String> {
  PANIC_MESSAGE.with(|slot| slot.borrow_mut().take())
}

/// Marks the StyleX error about to be reported as a plain `Error` with no code frame, as in Babel.
pub fn mark_panic_plain() {
  PANIC_PLAIN.with(|plain| plain.set(true));
}

/// Whether the last StyleX error on this thread was marked plain, cleared.
pub fn take_panic_plain() -> bool {
  PANIC_PLAIN.with(|plain| plain.replace(false))
}

/// Records the span of the node the next StyleX error will be reported at.
pub fn record_panic_span(lo: u32, hi: u32) {
  PANIC_SPAN.with(|span| span.set(Some((lo, hi))));
}

/// The span recorded for the last StyleX error on this thread, cleared.
pub fn take_panic_span() -> Option<(u32, u32)> {
  PANIC_SPAN.with(|span| span.take())
}

/// Runs `work` with its own error-reporting state, restoring the caller's state afterwards.
pub fn with_isolated_error_state<R>(work: impl FnOnce() -> R) -> R {
  struct Restore(Option<(u32, u32)>, bool, Option<String>, Option<String>);
  impl Drop for Restore {
    fn drop(&mut self) {
      PANIC_DECLARATION.with(|slot| *slot.borrow_mut() = self.3.take());
      PANIC_SPAN.with(|span| span.set(self.0));
      PANIC_PLAIN.with(|plain| plain.set(self.1));
      PANIC_MESSAGE.with(|slot| *slot.borrow_mut() = self.2.take());
    }
  }
  let _restore = Restore(
    PANIC_SPAN.with(|span| span.take()),
    PANIC_PLAIN.with(|plain| plain.replace(false)),
    PANIC_MESSAGE.with(|slot| slot.borrow_mut().take()),
    PANIC_DECLARATION.with(|slot| slot.borrow_mut().take()),
  );
  work()
}

/// Extracts a plain-text message from a caught panic payload, adding `[StyleX]` if it is missing.
pub fn format_panic_message(error: &Box<dyn std::any::Any + Send>) -> String {
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

/// Renders every diagnostic this module colours as plain text, process-wide.
/// Call it before asserting on rendered output, because colouring depends on the terminal.
pub fn disable_colour_output() {
  colored::control::set_override(false);
}

// Panic-output suppression, used around `catch_unwind` at the NAPI boundary.

thread_local! {
  static SUPPRESS_PANIC_STDERR: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
#[path = "tests/stylex_error_test.rs"]
mod tests;
