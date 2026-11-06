//! Ported normalizer 7 of 9 in upstream's order; `normalize_value.rs` has the run order.

use postcss_value_parser::{NodeKind, ValueParser};

/// Gives an empty string a double quote, so `''` and `` hash alike; nothing else changes.
pub fn normalize_quotes(ast: &mut ValueParser, _key: &str) {
  ast.walk(
    |node, _| {
      if node.kind != NodeKind::String {
        return true;
      }

      if node.value.is_empty() {
        node.quote = Some('"');
      }

      true
    },
    false,
  );
}
