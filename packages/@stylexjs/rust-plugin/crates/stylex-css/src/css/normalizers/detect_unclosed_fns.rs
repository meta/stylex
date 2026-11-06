//! Ported normalizer 1 of 9 in upstream's sequence, which is what that count
//! names. See `normalize_value.rs` for the order the passes run in here — a
//! tenth pass that upstream has no equivalent for runs among them.

use postcss_value_parser::{NodeKind, ValueParser};
use stylex_constants::constants::messages::LINT_UNCLOSED_FUNCTION;

use super::reject_value;

/// Rejects a value carrying a function that was never closed.
///
/// Runs first so that no later normalizer gets to rewrite tokens the author
/// never finished writing.
///
/// The reference implementation raises a bare message here; the rule text is
/// appended so the report names the declaration it came from, which is what
/// this compiler's message has always carried.
///
/// That text is read back off the token list rather than kept from the input,
/// which nothing has yet rewritten — this runs first. The one value that comes
/// back spelled differently is the scanner's documented `/*/`, so a rejected
/// value containing that quotes itself slightly repaired. It is a message, and
/// only a message: no value that reaches a stylesheet passes through here.
pub fn detect_unclosed_fns(ast: &mut ValueParser, key: &str) {
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
    reject_value(ast, key, LINT_UNCLOSED_FUNCTION);
  }
}
