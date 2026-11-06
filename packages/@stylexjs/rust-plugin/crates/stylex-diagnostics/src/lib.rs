//! How StyleX shows an author where a refusal happened.
//!
//! A code frame quotes the offending line back out of the file the author
//! wrote, which means finding that line again: what the compiler holds by then
//! is a rewritten tree whose positions belong to its own source map, not to the
//! text on disk. So the module is re-read, re-parsed into the frame's own map,
//! and searched -- for the expression itself, for the namespace key that names
//! it, or for the declaration of the binding the refusal is really about.
//!
//! All of it is best effort. Every lookup here sits behind a panic boundary and
//! degrades to "no code frame", because a compilation must never stop on
//! account of the aid that explains why it stopped.

pub mod code_frame;
pub(crate) mod declaration_span;
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
