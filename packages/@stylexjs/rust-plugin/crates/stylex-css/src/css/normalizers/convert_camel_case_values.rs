//! Ported normalizer 8 of 9 in upstream's order; `normalize_value.rs` has the run order.

use std::borrow::Cow;

use postcss_value_parser::{NodeKind, ValueParser};
use stylex_utils::string::dashify;

/// Rewrites a camel-cased property name in a value (`transitionProperty: 'backgroundColor'`)
/// into CSS spelling.
///
/// Only `transitionProperty` and `willChange`, top-level words only; custom properties keep case.
pub fn convert_camel_cased_values(ast: &mut ValueParser, key: &str) {
  if key != "transitionProperty" && key != "willChange" {
    return;
  }

  for node in &mut ast.nodes {
    if node.kind == NodeKind::Word && !node.value.starts_with("--") {
      // Only an owned result is a rewrite; a borrowed one is unchanged.
      if let Cow::Owned(dashed) = dashify(&node.value) {
        node.value = dashed;
      }
    }
  }
}
