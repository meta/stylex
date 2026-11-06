//! A logger the tests can read back, so the reporting paths are covered by
//! assertions rather than by a coverage exclusion.
//!
//! Both the level and the captured messages are per thread, because the test
//! harness runs tests in parallel and a global level would let one test decide
//! what another one sees. `log::max_level` stays wide open; the per-thread level
//! is what [`Log::enabled`] answers from, which is what `log_enabled!` asks.

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

/// Installs the logger. Called once, from the test harness's own start-up.
///
/// Through a `Once`, so the two steps cannot come apart. `set_boxed_logger`
/// answers a second caller `Err`, and a caller that read `Err` and went on
/// would log against a maximum level the winner had not raised yet -- `log`
/// builds no message at all below that level, so the case read an empty list
/// back and reported it against the code under test.
///
/// A refused install is reported by every caller rather than by the one that
/// met it. `log` takes one logger for the whole process, so a foreign one that
/// won means this logger captures nothing and each case reads an empty list
/// back -- the symptom above, with nothing left to notice it by. Recorded and
/// then asserted outside the `Once`, because a panic raised inside it poisons
/// the `Once` and leaves the next caller reading about that instead.
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

/// Runs `body` with this thread logging at `level`, and hands back everything it
/// logged.
///
/// The level goes back to the thread's own however the body leaves, a panic
/// included. The harness runs the cases of one binary on a pool of threads and
/// reuses them, so a level left open by a failing case would have the next case
/// on that thread keep messages nothing asked for. The panic itself still
/// travels out, because a case that fails unexpectedly has to say so rather
/// than go on to an assertion about a log.
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
