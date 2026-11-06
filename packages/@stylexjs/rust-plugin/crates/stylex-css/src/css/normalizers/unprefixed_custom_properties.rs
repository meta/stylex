//! Local pass, not a port: rejects `var(foo)` missing its `--`, which upstream emits verbatim.
//! It only changes which programs are rejected, never an accepted value's bytes.

use postcss_value_parser::{Node, NodeKind, ValueParser};
use stylex_constants::constants::messages::UNPREFIXED_CUSTOM_PROPERTIES;

use super::reject_value;

/// The function name a custom-property reference is spelled with; matched case-sensitively.
const CUSTOM_PROPERTY_FN: &str = "var";

/// The prefix every custom property name carries.
const CUSTOM_PROPERTY_PREFIX: &str = "--";

/// The unprefixed property name `node` references, if it is a top-level `var()` whose first
/// argument is a word without `--`.
fn unprefixed_property_name(node: &Node) -> Option<&str> {
  if node.kind != NodeKind::Function || node.value != CUSTOM_PROPERTY_FN {
    return None;
  }

  let first = node.nodes.as_ref().and_then(|nodes| nodes.first())?;

  match first.kind == NodeKind::Word && !first.value.starts_with(CUSTOM_PROPERTY_PREFIX) {
    true => Some(first.value.as_str()),
    false => None,
  }
}

/// Rejects a top-level custom-property reference whose name lacks its `--` prefix.
///
/// Runs after the unclosed detectors (so `var(foo` reports the unclosed function) and before any
/// rewrite (so the quoted name is as typed).
pub fn detect_unprefixed_custom_properties(ast: &mut ValueParser, key: &str) {
  if let Some(name) = ast.nodes.iter().find_map(unprefixed_property_name) {
    let message = format!("{UNPREFIXED_CUSTOM_PROPERTIES}: var({name})");

    reject_value(ast, key, &message);
  }
}

#[cfg(test)]
#[path = "../../tests/unprefixed_custom_properties_predicate_tests.rs"]
mod tests;
