//! A logger tests can read back, so log messages are asserted rather than excluded from
//! coverage. Level and messages are per thread because tests run in parallel. Duplicates
//! `stylex_diagnostics`'s helper since `log` allows one logger per process.

use std::cell::RefCell;
use std::sync::Once;
use std::sync::atomic::{AtomicBool, Ordering};

use log::{Level, LevelFilter, Log, Metadata, Record};

thread_local! {
  static LEVEL: RefCell<LevelFilter> = const { RefCell::new(LevelFilter::Off) };
  static MESSAGES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

struct CapturingLogger;

impl Log for CapturingLogger {
  fn enabled(&self, metadata: &Metadata) -> bool {
    LEVEL.with_borrow(|level| metadata.level() <= *level)
  }

  fn log(&self, record: &Record) {
    if !self.enabled(record.metadata()) {
      return;
    }

    MESSAGES.with_borrow_mut(|messages| messages.push(record.args().to_string()));
  }

  fn flush(&self) {}
}

/// Installs the logger once; a refused install (a foreign logger won) is asserted by every
/// caller, outside the `Once` so a panic doesn't poison it.
fn install() {
  static INSTALLED: Once = Once::new();
  static REFUSED: AtomicBool = AtomicBool::new(false);

  INSTALLED.call_once(|| {
    if log::set_boxed_logger(Box::new(CapturingLogger)).is_ok() {
      log::set_max_level(LevelFilter::Trace);
    } else {
      REFUSED.store(true, Ordering::Relaxed);
    }
  });

  assert!(
    !REFUSED.load(Ordering::Relaxed),
    "this test binary could not install the logger its cases read back"
  );
}

/// Runs `body` logging at `level` on this thread and returns what it wrote.
/// The level is reset even on panic (threads are reused); the panic still propagates.
pub(crate) fn logged_at<T>(level: Level, body: impl FnOnce() -> T) -> Vec<String> {
  struct Closed;

  impl Drop for Closed {
    fn drop(&mut self) {
      LEVEL.with_borrow_mut(|current| *current = LevelFilter::Off);
    }
  }

  install();

  LEVEL.with_borrow_mut(|current| *current = level.to_level_filter());
  MESSAGES.with_borrow_mut(Vec::clear);

  let _closed = Closed;

  body();

  MESSAGES.with_borrow(Clone::clone)
}
