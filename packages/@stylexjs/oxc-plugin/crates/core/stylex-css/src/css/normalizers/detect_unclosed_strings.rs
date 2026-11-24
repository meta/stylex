//! Ported normalizer 2 of 9 in upstream's order; `normalize_value.rs` has the run order.

use postcss_value_parser::{NodeKind, ValueParser};
use stylex_constants::constants::messages::LINT_UNCLOSED_STRING;

use stylex_macros::stylex_panic;

/// Rejects a value carrying a string that was never closed, with Babel's bare message.
///
/// The scanner invents the missing quote, so this is the only check for an unterminated string.
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
