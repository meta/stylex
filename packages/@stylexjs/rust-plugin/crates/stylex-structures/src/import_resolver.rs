use std::fmt;

/// Resolves theme file imports like the host's StyleX configuration; without one, `commonJS`
/// resolves natively and `custom` resolves nothing.
pub trait ImportResolver: fmt::Debug {
  /// The canonical path a theme import names (Babel's `getCanonicalFilePath`), or `None`.
  /// `Err` carries a host failure the host reports itself.
  fn resolve_theme_import(
    &self,
    import_path: &str,
    source_file_path: &str,
  ) -> Result<Option<String>, String>;

  /// `custom` resolution: the host's canonical name for `file_path`; `None` keeps the compiler's.
  fn canonical_file_path(&self, _file_path: &str) -> Result<Option<String>, String> {
    Ok(None)
  }

  /// `rewriteAliases`: the relative specifier a theme import is rewritten to, or `None`.
  fn rewrite_alias_import(
    &self,
    _import_path: &str,
    _source_file_path: &str,
  ) -> Result<Option<String>, String> {
    Ok(None)
  }
}

/// Names a file in `$$css` debug labels like the host's `debugFilePath` (`''` writes no label).
pub trait DebugFileNamer: fmt::Debug {
  fn debug_file_name(&self, absolute_path: &str) -> Result<String, String>;
}
