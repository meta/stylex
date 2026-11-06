//! A loose CSS declaration-value scanner: words, spaces, separators, strings,
//! comments and functions, each keeping the characters the author typed.
//!
//! Third-party code. Copyright (c) Bogdan Chadkin <trysound@yandex.ru>,
//! `postcss-value-parser` v4.2.0 at revision `43ae6d3`. MIT licensed; the full
//! notice ships beside this crate in `LICENSE`, the repository's
//! [`NOTICE.md`](../../NOTICE.md) lists it alongside everything else here that
//! somebody else wrote, and the behaviour is that library's, quirks and all.
//!
//! It has a crate to itself for that reason. It is somebody else's work, it is
//! left reading the way it was written rather than idiomatised, and a crate
//! boundary says so in a way a directory name cannot: nothing in here is StyleX
//! logic, and nothing in here depends on anything that is. It has no
//! dependencies at all.
//!
//! ## What it is for
//!
//! A StyleX class name is a hash of the canonical declaration text, and this
//! scanner's serialised output *is* that text. Its looseness is the point: a
//! token here is an arbitrary blob rather than a spec-conformant CSS token, so
//! a percentage, a signed dimension and a number in exponent notation are each
//! one token, and every character not deliberately rewritten survives byte for
//! byte — hex spelling, letter case, quote character, whitespace positions.
//!
//! It never rejects. Unclosed functions, unclosed strings and unterminated
//! comments are recorded as flags on the node they belong to, which is what
//! lets a value written in syntax newer than this compiler's knowledge pass
//! through unharmed. There is no error type here and no input that produces
//! one.
//!
//! ## The one way it can still fail
//!
//! Depth. The scan itself is iterative and parses `"(".repeat(200_000)`
//! without complaint, but spelling the result back out, walking it, and
//! dropping it all recurse once per level — so a tree nested deeply enough
//! exhausts the stack. In Rust that is an abort, not something a caller can
//! catch, where the JavaScript throws a `RangeError`.
//!
//! An embedder must therefore bound nesting depth *before* handing a value
//! here. `stylex-css` does, rejecting past 64 in `scan_value_structure`, which
//! is why nothing in this workspace can reach the abort. Lifting that bound, or
//! exposing this crate outside the workspace, means rewriting the three
//! recursive walks against an explicit stack first.
//!
//! ## Where a round trip does not hold
//!
//! Serialising a freshly parsed value normally reproduces the input byte for
//! byte. `/*/` is the exception: the comment scan starts at the opening `/`
//! rather than past it, finds its `*/` terminator inside the `/*/` itself, and
//! so `/*/ x */` comes back as `/**/ x */`. Two more degenerate shapes push a
//! source offset one byte past the end of the input — an unclosed string, whose
//! invented closing quote extends the buffer offsets are measured against, and
//! a trailing backslash, which makes the word scan overshoot. All three are
//! deliberate: correcting any of them would change class names.
//!
//! ## Why exports with no caller stay
//!
//! [`stringify_with`], [`stringify_node_with`] and [`Custom`] have no caller
//! outside this crate's own tests, and [`ValueParser::walk`] returns `&mut
//! Self` that nobody chains. They are upstream's surface, kept whole and
//! covered so this crate answers the same questions the library does. Read
//! them as the port's boundary, not as generality added on spec: a missing one
//! is a difference from the thing being ported, which is the one property this
//! crate exists to hold.
//!
//! ## Two differences from the JavaScript
//!
//! The word scanner's second `code === slash` test, guarded by `parent.type`,
//! sits behind an unguarded `code === slash` that already matched — it is dead,
//! and reaching it would throw on an undefined parent. It is left out rather
//! than reproduced as a latent panic.
//!
//! A walk callback is handed the node and its index, but not the list holding
//! it: Rust cannot lend out both at once. Nothing reads that third argument —
//! not the JavaScript's own tests, and not the normalizers, which reach the
//! list they want to restructure directly instead. [`walk`] spells out what
//! follows from that.

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
///
/// The string spellings are kept by [`NodeKind::as_str`], so that a node can
/// be named in a message or a dump the way the JavaScript names it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
  /// Anything that is not one of the shapes below: an identifier, a number, a
  /// dimension, a hex colour, a `!important` annotation, or an operator inside
  /// `calc()`. Deliberately coarse.
  Word,
  /// A quoted string. `value` holds the text *between* the quotes, exactly as
  /// written, escapes included.
  String,
  /// A separator — `,`, `:` or `/` — with the whitespace around it captured
  /// rather than emitted as separate nodes.
  Div,
  /// A run of one or more whitespace characters that no other node claimed.
  Space,
  /// A `/* ... */` comment. `value` holds the text between the delimiters.
  Comment,
  /// A function call. `value` is the name, `nodes` the arguments.
  Function,
  /// A `U+xxxx` style range. Recognised only so that it is not mistaken for a
  /// word followed by a signed number.
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

/// One parsed token.
///
/// This is a single record with a kind discriminant and optional fields rather
/// than an enum, and that is a deliberate trade. The normalizers built on top
/// are each written as "inspect the kind, then assign to the value field"; an
/// enum would force every one of them into a match with a catch-all arm,
/// restructuring code that is meant to read the way it was written.
///
/// Each optional field below documents which kinds populate it. A field left
/// `None` is the JavaScript's `undefined`, not an empty string — the two are
/// distinguishable here for the same reason they are there.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Node {
  /// Which shape this node is.
  pub kind: NodeKind,
  /// The node's text, read differently per kind: the raw source text for
  /// [`NodeKind::Word`], [`NodeKind::Space`] and [`NodeKind::UnicodeRange`];
  /// the text inside the delimiters for [`NodeKind::String`] and
  /// [`NodeKind::Comment`]; the separator character for [`NodeKind::Div`]; the
  /// function name for [`NodeKind::Function`].
  pub value: String,
  /// Whitespace preceding the token's own characters. Populated for
  /// [`NodeKind::Div`] (whitespace before the separator) and
  /// [`NodeKind::Function`] (whitespace just inside the opening parenthesis).
  pub before: Option<String>,
  /// Whitespace following the token's own characters. Populated for
  /// [`NodeKind::Div`] (whitespace after the separator) and
  /// [`NodeKind::Function`] (whitespace just inside the closing parenthesis).
  pub after: Option<String>,
  /// The quote character the author used. Populated for [`NodeKind::String`]
  /// only, and preserved so that a single-quoted string stays single-quoted.
  pub quote: Option<char>,
  /// Set when the token ran off the end of the input: a string with no closing
  /// quote, a comment with no `*/`, or a function with no `)`. Populated for
  /// [`NodeKind::String`], [`NodeKind::Comment`] and [`NodeKind::Function`];
  /// always `false` for the rest.
  pub unclosed: bool,
  /// The arguments between the parentheses. Populated for
  /// [`NodeKind::Function`] only — including `url()`, whose body is a single
  /// word node rather than a parsed argument list.
  pub nodes: Option<Vec<Node>>,
  /// Byte offset where the token starts in the input. Populated for every kind.
  ///
  /// Load-bearing, not bookkeeping: the zero-dimension normalizer decides
  /// whether a token sits inside a function by comparing offsets rather than by
  /// tracking visitor state.
  pub source_index: usize,
  /// Byte offset just past where the token ends. Populated for every kind.
  ///
  /// For an unclosed token this points at the end of the input rather than past
  /// a delimiter that was never written.
  pub source_end_index: usize,
}

impl Node {
  /// A node of `kind` carrying `value` and spanning `source_index ..
  /// source_end_index`, with every optional field absent.
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

/// A parsed value: the node list, plus the operations that hang off it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueParser {
  /// The top-level token list.
  pub nodes: Vec<Node>,
}

impl ValueParser {
  /// Parses `value`. Never fails — see the module documentation.
  pub fn new(value: &str) -> Self {
    ValueParser {
      nodes: parse(value),
    }
  }

  /// Visits every node, descending into functions.
  ///
  /// `cb` receives each node and its index among its siblings, and returns
  /// whether to descend into it. Returning `false` from a function node's visit
  /// skips its children; the return value is ignored when `bubble` is set,
  /// because the JavaScript never consults it on that path.
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
