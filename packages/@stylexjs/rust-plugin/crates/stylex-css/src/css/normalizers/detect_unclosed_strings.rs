//! Ported normalizer 2 of 9 in upstream's sequence, which is what that count
//! names. See `normalize_value.rs` for the order the passes run in here — a
//! tenth pass that upstream has no equivalent for runs among them.

use postcss_value_parser::{NodeKind, ValueParser};
use stylex_constants::constants::messages::LINT_UNCLOSED_STRING;

use stylex_macros::stylex_panic;

/// Rejects a value carrying a string that was never closed.
///
/// The scanner invents the missing closing quote rather than failing, so this
/// is the only thing standing between an unterminated string and a declaration
/// that swallows whatever followed it.
///
/// The public compatibility diagnostic is the same bare message as Babel;
/// source/code-frame context is added by the owning host layer.
pub fn detect_unclosed_strings(ast: &mut ValueParser, _key: &str) {
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
    stylex_panic!("{}", LINT_UNCLOSED_STRING);
  }
}
