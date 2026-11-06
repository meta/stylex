//! Ported normalizer 2 of 9 in upstream's sequence, which is what that count
//! names. See `normalize_value.rs` for the order the passes run in here — a
//! tenth pass that upstream has no equivalent for runs among them.

use postcss_value_parser::{NodeKind, ValueParser};
use stylex_constants::constants::messages::LINT_UNCLOSED_STRING;

use super::reject_value;

/// Rejects a value carrying a string that was never closed.
///
/// The scanner invents the missing closing quote rather than failing, so this
/// is the only thing standing between an unterminated string and a declaration
/// that swallows whatever followed it.
///
/// Rejects through [`super::reject_value`], the same way [`super::detect_unclosed_fns`]
/// does, so the two unfinished-construct reports quote the rule identically.
/// That is why this reads the key, which it otherwise has no use for.
pub fn detect_unclosed_strings(ast: &mut ValueParser, key: &str) {
  let mut unclosed = false;

  ast.walk(
    |node, _| {
      if node.kind == NodeKind::String && node.unclosed {
        unclosed = true;
      }

      true
    },
    false,
  );

  if unclosed {
    reject_value(ast, key, LINT_UNCLOSED_STRING);
  }
}
