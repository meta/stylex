//! A logger tests can read back, so reporting paths run and are asserted. It wraps this
//! binary's printing logger (`log` allows one per process) and keeps a per-thread level and
//! message list; `log::max_level` stays wide open so every message is built.

use std::cell::RefCell;
use std::sync::Once;
use std::sync::atomic::{AtomicBool, Ordering};

use log::{Level, LevelFilter, Log, Metadata, Record};

thread_local! {
  static LEVEL: RefCell<LevelFilter> = const { RefCell::new(LevelFilter::Off) };
  static MESSAGES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// Everything the wrapped logger writes, plus a copy of what this thread keeps.
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

/// Installs the logger, wrapping the one this binary prints with. Runs once from a `ctor`;
/// `max_level` is process-wide, so it is opened for the whole binary, not per case.
/// Through a `Once` so a second caller cannot log before the level is raised; a refused install
/// is asserted outside the `Once` by every caller, since a panic inside would poison it.
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

/// Runs `body` with this thread keeping messages at `level`, and hands back what it wrote.
/// The level closes even on panic (test threads are reused). Work on another thread is not
/// captured.
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
