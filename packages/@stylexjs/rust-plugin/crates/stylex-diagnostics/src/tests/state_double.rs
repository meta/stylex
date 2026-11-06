//! A stand-in for the transform's state manager, for testing the diagnostics in isolation.
//!
//! Replacing the memoized module drops the key span index built from it.

use std::{cell::OnceCell, sync::Arc};

use swc_core::common::SourceFile;
use swc_core::ecma::ast::Module;

use crate::{memo::DiagnosticMemo, state::DiagnosticState};
use stylex_state_index::key_span_index::KeySpanIndex;

#[derive(Default)]
pub(crate) struct StateDouble {
  filename: String,
  seen_module: Option<Module>,
  seen_source_file: Option<Arc<SourceFile>>,
  input_source: Option<String>,
  /// False by default, because `useRealFileForSource` defaults to on.
  disk_reads_off: bool,
  key_span_index: OnceCell<KeySpanIndex>,
  memo: DiagnosticMemo,
  /// Drops the module as soon as it is stored, like a state that failed to memoize it.
  forgets_the_module: bool,
}

impl StateDouble {
  /// A state that names `filename` as the file being transformed.
  pub(crate) fn for_file(filename: impl Into<String>) -> Self {
    Self {
      filename: filename.into(),
      ..Self::default()
    }
  }

  /// The same state, holding the text the compiler was given for the file.
  pub(crate) fn with_input_source(self, source: impl Into<String>) -> Self {
    Self {
      input_source: Some(source.into()),
      ..self
    }
  }

  /// The same state with `useRealFileForSource` off, so the frame opens no file.
  pub(crate) fn with_disk_reads_off(self) -> Self {
    Self {
      disk_reads_off: true,
      ..self
    }
  }

  /// A state that never remembers the module it was given.
  pub(crate) fn forgetful() -> Self {
    Self {
      forgets_the_module: true,
      ..Self::default()
    }
  }
}

impl DiagnosticState for StateDouble {
  fn get_filename(&self) -> &str {
    &self.filename
  }

  fn get_seen_module_source_code(&self) -> Option<(&Module, Option<&str>)> {
    Some((
      self.seen_module.as_ref()?,
      self.seen_source_file.as_ref().map(|file| file.src.as_str()),
    ))
  }

  fn set_seen_module_source_code(&mut self, module: &Module, source_file: Option<Arc<SourceFile>>) {
    if self.forgets_the_module {
      return;
    }

    self.seen_module = Some(module.clone());
    self.seen_source_file = source_file;
    // Built from the module above, so a new module invalidates it.
    self.key_span_index = OnceCell::new();
  }

  fn key_span_index(&self) -> Option<&KeySpanIndex> {
    let module = self.seen_module.as_ref()?;

    Some(
      self
        .key_span_index
        .get_or_init(|| KeySpanIndex::build(module)),
    )
  }

  fn input_source_text(&self) -> Option<&str> {
    self.input_source.as_deref()
  }

  fn reads_source_from_disk(&self) -> bool {
    !self.disk_reads_off
  }

  fn diagnostic_memo(&self) -> &DiagnosticMemo {
    &self.memo
  }

  fn diagnostic_memo_mut(&mut self) -> &mut DiagnosticMemo {
    &mut self.memo
  }
}
