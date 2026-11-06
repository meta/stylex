//! The per-file memo of spans and declarations that diagnostics have already resolved.
//!
//! Keys are 128-bit hashes because a hit is used without verification, so a collision would
//! show the wrong line.
//! width is the only thing standing behind it.

use rustc_hash::FxHashMap;
use swc_core::{atoms::Atom, common::Span};

/// The spans and framed declarations that diagnostics resolved for one file.
#[derive(Clone, Debug, Default)]
pub struct DiagnosticMemo {
  spans: FxHashMap<u128, Span>,
  /// Per refused expression, the binding whose declaration its code frame should point at.
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
  pub fn has_framed_declarations(&self) -> bool {
    !self.framed_declarations.is_empty()
  }
}

#[cfg(test)]
#[path = "tests/memo_test.rs"]
mod tests;
