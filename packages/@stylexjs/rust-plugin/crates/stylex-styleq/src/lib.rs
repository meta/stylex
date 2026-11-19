//! Compile-time port of the StyleX runtime's style merge: last write wins per CSS property,
//! with an inline-style fallback. It must match the runtime exactly.
//!
//! Third-party work. Copyright (c) Nicolas Gallagher,
//! [`styleq`](https://github.com/necolas/styleq). MIT licensed; see `LICENSE` and
//! [`NOTICE.md`](../../NOTICE.md).

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
