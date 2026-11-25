//! Ported normalizer 1 of 9 in upstream's order; `normalize_value.rs` has the run order.

use postcss_value_parser::{NodeKind, ValueParser};
use stylex_constants::constants::messages::LINT_UNCLOSED_FUNCTION;

use stylex_macros::stylex_panic;

/// Rejects a value carrying a function that was never closed, with Babel's bare message.
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
