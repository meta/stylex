//! Code frames that show an author where StyleX refused a style.
//!
//! Every lookup is best effort: it sits behind a panic boundary and degrades to no code frame,
//! so a diagnostic never stops a compilation.

pub mod code_frame;
pub mod declaration_span;
pub mod memo;
pub mod state;

/// The state manager stand-in the tests read diagnostics through.
#[cfg(test)]
#[path = "tests/state_double.rs"]
mod state_double;

/// The logger the tests read the reporting paths back from.
#[cfg(test)]
#[path = "tests/capturing_logger.rs"]
mod capturing_logger;

#[cfg(test)]
#[ctor::ctor(unsafe)]
fn init_logger() {
  capturing_logger::install();
}
