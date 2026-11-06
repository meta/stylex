//! Ported normalizer 1 of 9 in upstream's sequence, which is what that count
//! names. See `normalize_value.rs` for the order the passes run in here — a
//! tenth pass that upstream has no equivalent for runs among them.

use postcss_value_parser::{NodeKind, ValueParser};
use stylex_constants::constants::messages::LINT_UNCLOSED_FUNCTION;

use stylex_macros::stylex_panic;

/// Rejects a value carrying a function that was never closed.
///
/// Runs first so that no later normalizer gets to rewrite tokens the author
/// never finished writing.
///
/// The compatibility diagnostic is Babel's bare message; source/code-frame
/// context is added by the owning host layer, not by reserializing a CSS rule.
pub fn detect_unclosed_fns(ast: &mut ValueParser, _key: &str) {
  let mut unclosed = false;

  ast.walk(
    |node, _| {
      if node.kind == NodeKind::Function && node.unclosed {
        unclosed = true;
      }

      true
    },
    false,
  );

  if unclosed {
    stylex_panic!("{}", LINT_UNCLOSED_FUNCTION);
  }
}
