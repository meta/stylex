//! What a diagnostic remembers about the file it is writing about.
//!
//! Both maps are the diagnostics' own, so they live here rather than in the
//! compilation state that stores them. The state holds one of these as a field
//! and hands it back through [`DiagnosticState`](crate::state::DiagnosticState);
//! it never reads or writes either map itself.
//!
//! Both are keyed by 128 bits, because the read side acts on a hit alone: a
//! cached span is turned straight into a `file:line`. A collision is directly
//! observable -- a style annotated with another style's line number -- so the
//! width is the only thing standing behind it.

use rustc_hash::FxHashMap;
use swc_core::{atoms::Atom, common::Span};

/// The positions a diagnostic already resolved in this file, and the bindings
/// its refusals are about.
///
/// One per file being transformed, because every key indexes a position in that
/// file.
#[derive(Clone, Debug, Default)]
pub struct DiagnosticMemo {
  spans: FxHashMap<u128, Span>,
  /// Per refused expression, the binding whose declaration its code frame
  /// should point at instead of the expression itself.
  ///
  /// Keyed by the expression, not held as one slot, because a refusal is not
  /// always the end of the build: a dynamic style's value falls through to an
  /// inline style, so a later, unrelated diagnostic must not inherit an earlier
  /// refusal's declaration.
  framed_declarations: FxHashMap<u128, Atom>,
}

impl DiagnosticMemo {
  /// Where a previous lookup put the answer for `cache_key`, if it asked.
  pub fn cached_span(&self, cache_key: u128) -> Option<Span> {
    self.spans.get(&cache_key).copied()
  }

  /// Records the answer for `cache_key`, replacing any earlier one.
  pub fn insert_cached_span(&mut self, cache_key: u128, span: Span) {
    self.spans.insert(cache_key, span);
  }

  /// Records that the refusal behind `cache_key` is about the binding `name`.
  pub fn frame_declaration(&mut self, cache_key: u128, name: Atom) {
    self.framed_declarations.insert(cache_key, name);
  }

  /// The binding recorded against `cache_key`, if one was.
  pub fn framed_declaration(&self, cache_key: u128) -> Option<&Atom> {
    self.framed_declarations.get(&cache_key)
  }

  /// Whether any refusal recorded a declaration to frame.
  ///
  /// False for every build that refuses nothing, which lets the annotation path
  /// answer without hashing an expression.
  pub fn has_framed_declarations(&self) -> bool {
    !self.framed_declarations.is_empty()
  }
}

#[cfg(test)]
#[path = "tests/memo_test.rs"]
mod tests;
