use std::sync::Arc;

use swc_core::common::SourceFile;
use swc_core::ecma::ast::Module;

use stylex_state_index::key_span_index::KeySpanIndex;

use crate::memo::DiagnosticMemo;

/// The compiler state a diagnostic reads, implemented by the `stylex_state` state manager.
///
/// Take it by generic bound, not `dyn`: the source-map annotation path calls it in a hot loop.
pub trait DiagnosticState {
  /// The file being transformed, as the frame should name it.
  fn get_filename(&self) -> &str;

  /// The re-parsed module memoized by an earlier diagnostic, with its source text if known.
  fn get_seen_module_source_code(&self) -> Option<(&Module, Option<&str>)>;

  /// Memoizes the re-parsed module and the source file it was parsed from, if any.
  fn set_seen_module_source_code(&mut self, module: &Module, source_file: Option<Arc<SourceFile>>);

  /// The text the compiler was given for this file, if the entry point recorded it.
  fn input_source_text(&self) -> Option<&str>;

  /// Whether the frame may read the file on disk, as `useRealFileForSource` asks.
  /// When false, [`DiagnosticState::input_source_text`] must not be used either.
  fn reads_source_from_disk(&self) -> bool;

  /// The index of every style namespace key in the memoized source, built on first use.
  fn key_span_index(&self) -> Option<&KeySpanIndex>;

  /// What the diagnostics remembered about this file so far.
  fn diagnostic_memo(&self) -> &DiagnosticMemo;

  /// The same memo, to write the next answer into.
  fn diagnostic_memo_mut(&mut self) -> &mut DiagnosticMemo;
}
