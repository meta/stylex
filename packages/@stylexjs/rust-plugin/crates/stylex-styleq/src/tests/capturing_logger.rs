//! A logger tests can read back, so log messages are asserted rather than excluded from
//! coverage. Level and messages are per thread because tests run in parallel. Duplicates
//! `stylex_diagnostics`'s helper since `log` allows one logger per process.

use std::cell::RefCell;

use log::LevelFilter;

thread_local! {
  static LEVEL: RefCell<LevelFilter> = const { RefCell::new(LevelFilter::Off) };
  static MESSAGES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}
