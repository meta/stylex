use std::sync::Arc;

use swc_core::common::SourceFile;
use swc_core::ecma::ast::Module;

use stylex_state_index::key_span_index::KeySpanIndex;

use crate::memo::DiagnosticMemo;

/// Everything a diagnostic has to read off the compiler's traversal state.
///
/// Owned here rather than in the state crate so that building a code frame
/// never names the state manager, which would make the state crate and the
/// diagnostics depend on each other. `stylex_state` implements the trait on its
/// state manager; this crate only ever sees these questions.
///
/// What a diagnostic remembers is not among them: the span and
/// framed-declaration maps are the diagnostics' own, so they are a
/// [`DiagnosticMemo`] the state merely stores. What is left is the compilation
/// state a frame cannot reconstruct -- the file being transformed and the module
/// re-parsed from it.
///
/// Every parameter that takes this trait takes it by generic bound, never as
/// `dyn`. The source-map annotation path asks these questions once per style
/// namespace, so a vtable here would put an indirect call on a hot loop.
pub trait DiagnosticState {
  /// The file being transformed, as the frame should name it.
  fn get_filename(&self) -> &str;

  /// The module's own source, re-parsed and memoized by an earlier diagnostic,
  /// together with the text it was parsed from -- which a module memoized
  /// without its text does not have.
  fn get_seen_module_source_code(&self) -> Option<(&Module, Option<&str>)>;

  /// Memoizes that re-parsed source, so the next diagnostic in the same file
  /// does not read and parse it again.
  ///
  /// The text comes as the source file it was parsed from, which the frame has
  /// already registered. Holding that file rather than a `String` keeps the
  /// module text to one allocation per file: the state and the source map share
  /// it. A module memoized without its text passes `None`.
  fn set_seen_module_source_code(&mut self, module: &Module, source_file: Option<Arc<SourceFile>>);

  /// The text the compiler was given for this file, when the entry point
  /// recorded it.
  ///
  /// It keeps the authored layout, which a module printed back out from its
  /// AST does not, so it is what a `file:line` is measured against when the
  /// file itself is not on disk.
  fn input_source_text(&self) -> Option<&str>;

  /// Whether the frame may read the file on disk, as `useRealFileForSource`
  /// asks. When the option is off, the frame quotes what the compiler holds in
  /// memory and opens no file.
  ///
  /// The flag gates [`DiagnosticState::input_source_text`] as well, not the
  /// disk read alone: with the option off, the module the compiler memoized is
  /// the one source the frame is allowed to name.
  fn reads_source_from_disk(&self) -> bool;

  /// Where every style namespace key of the memoized source is written, built
  /// on first use.
  fn key_span_index(&self) -> Option<&KeySpanIndex>;

  /// What the diagnostics remembered about this file so far.
  fn diagnostic_memo(&self) -> &DiagnosticMemo;

  /// The same memo, to write the next answer into.
  fn diagnostic_memo_mut(&mut self) -> &mut DiagnosticMemo;
}
