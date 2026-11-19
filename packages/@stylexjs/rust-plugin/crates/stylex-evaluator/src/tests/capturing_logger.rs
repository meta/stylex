//! Captures log messages per test thread while preserving the wrapped logger.

use std::cell::RefCell;
use std::sync::Once;
use std::sync::atomic::{AtomicBool, Ordering};

use log::{Level, LevelFilter, Log, Metadata, Record};

thread_local! {
  static LEVEL: RefCell<LevelFilter> = const { RefCell::new(LevelFilter::Off) };
  static MESSAGES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

struct CapturingLogger(Box<dyn Log>);

impl CapturingLogger {
  fn kept(&self, metadata: &Metadata) -> bool {
    LEVEL.with_borrow(|level| metadata.level() <= *level)
  }
}

impl Log for CapturingLogger {
  fn enabled(&self, metadata: &Metadata) -> bool {
    self.kept(metadata) || self.0.enabled(metadata)
  }

  fn log(&self, record: &Record) {
    if self.kept(record.metadata()) {
      MESSAGES.with_borrow_mut(|messages| messages.push(record.args().to_string()));
    }

    self.0.log(record);
  }

  fn flush(&self) {
    self.0.flush();
  }
}

/// Installs the wrapper once and enables message construction at every log level.
pub(crate) fn install(prints: impl Log + 'static) {
  static INSTALLED: Once = Once::new();
  static REFUSED: AtomicBool = AtomicBool::new(false);

  INSTALLED.call_once(|| {
    if log::set_boxed_logger(Box::new(CapturingLogger(Box::new(prints)))).is_ok() {
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

/// Captures this thread's messages at `level` and resets capture even if `body` panics.
pub(crate) fn logged_at<T>(level: Level, body: impl FnOnce() -> T) -> Vec<String> {
  struct Closed;

  impl Drop for Closed {
    fn drop(&mut self) {
      LEVEL.with_borrow_mut(|current| *current = LevelFilter::Off);
    }
  }

  LEVEL.with_borrow_mut(|current| *current = level.to_level_filter());
  MESSAGES.with_borrow_mut(Vec::clear);

  let _closed = Closed;

  body();

  MESSAGES.with_borrow(Clone::clone)
}
