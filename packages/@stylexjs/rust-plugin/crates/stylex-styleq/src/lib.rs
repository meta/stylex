//! Merges compiled style objects into one class name, the way the StyleX
//! runtime does: last write wins on the underlying CSS property, with an
//! inline-style fallback for anything not compiled.
//!
//! Third-party work. Copyright (c) Nicolas Gallagher,
//! [`styleq`](https://github.com/necolas/styleq). MIT licensed; the full notice
//! ships beside this crate in `LICENSE`, and the repository's
//! [`NOTICE.md`](../../NOTICE.md) lists it alongside everything else here that
//! somebody else wrote.
//!
//! The algorithm is the runtime's, reproduced so that the merge can happen at
//! compile time — the bundle then carries a literal class string instead of a
//! call. Its behaviour has to match the runtime's exactly, so this is left
//! reading the way the original does rather than idiomatised.

mod styleq;
mod types;

/// The logger the cache cases read messages back from.
#[cfg(test)]
#[path = "tests/capturing_logger.rs"]
mod capturing_logger;

pub use styleq::{Styleq, create_styleq, styleq};
pub use types::{
  StyleMap, StyleValue, StyleqArgument, StyleqInput, StyleqOptions, StyleqResult, StyleqValue,
};
