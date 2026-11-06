//! A logger the tests can read back, so a reporting path is covered by an
//! assertion rather than by a coverage exclusion.
//!
//! A `log` macro builds its message only when `log::max_level` admits it, so
//! every argument of every message is unexecuted code until a logger is in
//! place. Reading the message back then makes the message itself the subject of
//! a case.
//!
//! Both the level and the messages are per thread, because the harness runs
//! tests in parallel and one global level would let one case decide what
//! another one sees. `log::max_level` stays wide open; the per-thread level is
//! what [`Log::enabled`] answers from.
//!
//! The same helper as `stylex_diagnostics`'s. `log` takes one logger for the
//! whole process and every test binary is a process of its own, so each crate
//! that reads its messages back holds its own copy.

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

/// Installs the logger, on the first case that reads a message back.
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

/// Runs `body` with this thread logging at `level`, and hands back everything
/// it wrote.
///
/// The level is closed however the body leaves, a panic included. The harness
/// runs the cases of one binary on a pool of threads and reuses them, so a
/// level left open by a failing case would have the next case on that thread
/// keep messages nothing asked for. The panic itself still travels out, because
/// a case that fails unexpectedly has to say so rather than go on to an
/// assertion about a log.
pub(crate) fn logged_at<T>(level: Level, body: impl FnOnce() -> T) -> Vec<String> {
  /// Closes the level again however the body left.
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
