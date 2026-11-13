//! A per-thread logger that tests can read back.
//!
//! Levels and messages are per thread because tests run in parallel; `log::max_level` stays open.

use std::cell::RefCell;
use std::sync::Once;
use std::sync::atomic::{AtomicBool, Ordering};

use log::{Level, LevelFilter, Log, Metadata, Record};

/// What a thread keeps when no case has asked for anything else.
const DEFAULT_LEVEL: LevelFilter = LevelFilter::Warn;

thread_local! {
  static LEVEL: RefCell<LevelFilter> = const { RefCell::new(DEFAULT_LEVEL) };
  static MESSAGES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

struct CapturingLogger;

impl Log for CapturingLogger {
  fn enabled(&self, metadata: &Metadata) -> bool {
    LEVEL.with(|level| metadata.level() <= *level.borrow())
  }

  fn log(&self, record: &Record) {
    if !self.enabled(record.metadata()) {
      return;
    }

    MESSAGES.with(|messages| messages.borrow_mut().push(record.args().to_string()));
  }

  fn flush(&self) {}
}

/// Installs the logger once per process.
/// Panics if another logger was already installed, since the cases would then capture nothing.
pub(crate) fn install() {
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

/// Runs `body` logging at `level` and returns everything it logged.
/// The thread's level is restored even if `body` panics; the panic still propagates.
pub(crate) fn logged_at<T>(level: Level, body: impl FnOnce() -> T) -> Vec<String> {
  /// Puts the level back however the body left.
  struct Closed;

  impl Drop for Closed {
    fn drop(&mut self) {
      LEVEL.with(|current| *current.borrow_mut() = DEFAULT_LEVEL);
    }
  }

  LEVEL.with(|current| *current.borrow_mut() = level.to_level_filter());
  MESSAGES.with(|messages| messages.borrow_mut().clear());

  let _closed = Closed;

  body();

  MESSAGES.with(|messages| messages.borrow().clone())
}
