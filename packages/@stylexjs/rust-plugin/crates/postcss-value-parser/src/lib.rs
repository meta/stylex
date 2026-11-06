//! A port of `postcss-value-parser` v4.2.0, revision `43ae6d3`.
//! Copyright (c) Bogdan Chadkin <trysound@yandex.ru>. MIT licensed; see `LICENSE` and
//! [`NOTICE.md`](../../NOTICE.md).
//!
//! A loose, never-failing CSS declaration-value scanner whose serialized output must not change:
//! StyleX hashes that text into class names, so upstream's quirks are preserved byte for byte.
//!
//! Callers must bound nesting depth first: serializing, walking and dropping recurse per level
//! (`stylex-css` limits it to 64).

mod parse;
mod stringify;
mod unit;
mod walk;

#[cfg(test)]
mod tests;

pub use parse::parse;
pub use stringify::{Custom, stringify, stringify_node, stringify_node_with, stringify_with};
pub use unit::{Dimension, unit};
pub use walk::walk;

use std::fmt;

/// Which of the parser's seven token shapes a [`Node`] is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
  /// Anything not covered below: identifiers, numbers, dimensions, operators.
  Word,
  /// A quoted string; `value` is the text between the quotes, escapes included.
  String,
  /// A `,`, `:` or `/` separator, with surrounding whitespace in `before`/`after`.
  Div,
  /// Whitespace no other node claimed.
  Space,
  /// A `/* ... */` comment. `value` holds the text between the delimiters.
  Comment,
  /// A function call. `value` is the name, `nodes` the arguments.
  Function,
  /// A `U+xxxx` style range.
  UnicodeRange,
}

impl NodeKind {
  /// The `type` string for this kind.
  pub const fn as_str(self) -> &'static str {
    match self {
      NodeKind::Word => "word",
      NodeKind::String => "string",
      NodeKind::Div => "div",
      NodeKind::Space => "space",
      NodeKind::Comment => "comment",
      NodeKind::Function => "function",
      NodeKind::UnicodeRange => "unicode-range",
    }
  }
}

impl fmt::Display for NodeKind {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(self.as_str())
  }
}

/// One parsed token. A `None` field is the JavaScript's `undefined`, distinct from `""`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Node {
  pub kind: NodeKind,
  /// Source text; the inner text for strings and comments; the name for functions.
  pub value: String,
  /// Leading whitespace (divs; just inside `(` for functions).
  pub before: Option<String>,
  /// Trailing whitespace (divs; just inside `)` for functions).
  pub after: Option<String>,
  /// The quote character used (strings only).
  pub quote: Option<char>,
  /// The string, comment or function ran off the end of the input.
  pub unclosed: bool,
  /// Function arguments; `url()` holds a single word node.
  pub nodes: Option<Vec<Node>>,
  /// Start byte offset; normalizers compare offsets, so it must stay accurate.
  pub source_index: usize,
  /// End byte offset (exclusive); the end of input for an unclosed token.
  pub source_end_index: usize,
}

impl Node {
  /// A node with every optional field absent.
  pub fn new(kind: NodeKind, value: String, source_index: usize, source_end_index: usize) -> Self {
    Node {
      kind,
      value,
      before: None,
      after: None,
      quote: None,
      unclosed: false,
      nodes: None,
      source_index,
      source_end_index,
    }
  }
}

/// A parsed value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueParser {
  pub nodes: Vec<Node>,
}

impl ValueParser {
  /// Parses `value`; never fails.
  pub fn new(value: &str) -> Self {
    ValueParser {
      nodes: parse(value),
    }
  }

  /// Visits every node; `cb` returns whether to descend (ignored when `bubble` is set).
  pub fn walk<F>(&mut self, mut cb: F, bubble: bool) -> &mut Self
  where
    F: FnMut(&mut Node, usize) -> bool,
  {
    walk(&mut self.nodes, &mut cb, bubble);
    self
  }
}

impl fmt::Display for ValueParser {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&stringify(&self.nodes))
  }
}
