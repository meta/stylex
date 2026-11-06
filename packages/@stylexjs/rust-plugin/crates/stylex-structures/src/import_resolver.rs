use std::fmt;

/// Resolves a theme file import the way the embedding host's StyleX
/// configuration does.
///
/// The Node addon implements it over the JavaScript host, so `commonJS`
/// resolution and a `custom` `filePathResolver`/`getCanonicalFilePath` pair
/// answer exactly as they do in the Babel plugin. Without one, `commonJS`
/// resolves natively and `custom` resolves nothing.
pub trait ImportResolver: fmt::Debug {
  /// The canonical file path a theme import names (what the Babel plugin
  /// passes to `getCanonicalFilePath` after resolving it), or `None` when the
  /// import does not resolve. An `Err` carries a host failure; the host
  /// reports the exception itself.
  fn resolve_theme_import(
    &self,
    import_path: &str,
    source_file_path: &str,
  ) -> Result<Option<String>, String>;

  /// For `custom` module resolution: the canonical name of `file_path` as the
  /// host's `getCanonicalFilePath` answers it, which a theme file's own
  /// variables are hashed under. `None` leaves the compiler's own answer.
  fn canonical_file_path(&self, _file_path: &str) -> Result<Option<String>, String> {
    Ok(None)
  }

  /// For the `rewriteAliases` option: the relative specifier (`./theme/x.stylex`)
  /// a theme import is rewritten to, as the Babel plugin writes it after
  /// resolving the import through its aliases, or `None` when it does not
  /// resolve. Hosts that cannot answer leave imports as written.
  fn rewrite_alias_import(
    &self,
    _import_path: &str,
    _source_file_path: &str,
  ) -> Result<Option<String>, String> {
    Ok(None)
  }
}

/// Names a file in the `$$css` debug labels the way the host's
/// `debugFilePath` option does: given the file's absolute path, the name the
/// Babel plugin writes before `:<line>` (`''` writes no label).
pub trait DebugFileNamer: fmt::Debug {
  fn debug_file_name(&self, absolute_path: &str) -> Result<String, String>;
}
