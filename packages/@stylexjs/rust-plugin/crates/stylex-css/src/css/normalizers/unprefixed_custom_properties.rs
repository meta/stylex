//! Local pass, not a port. See `normalize_value.rs` for the ordered list.
//!
//! The reference compiler has no equivalent: it accepts `var(foo)` and emits it
//! verbatim. This pass is kept anyway because it changes only *which programs
//! are rejected* and never the bytes of an accepted one, so it cannot move a
//! class name, and because the mistake it catches — a custom property named
//! without its leading `--` — otherwise resolves to nothing at all in a browser
//! with no diagnostic from anywhere.

use postcss_value_parser::{Node, NodeKind, ValueParser};
use stylex_constants::constants::messages::UNPREFIXED_CUSTOM_PROPERTIES;

use super::reject_value;

/// The function name a custom-property reference is spelled with.
///
/// A browser reads function names case-insensitively, so `VAR(foo)` is the
/// same broken reference as `var(foo)`. This match is exact anyway, which is
/// what the rule has always done: it narrows the rule to the spelling every
/// author actually writes, and matching more widely would start rejecting
/// programs that compile today.
const CUSTOM_PROPERTY_FN: &str = "var";

/// The prefix every custom property name carries.
const CUSTOM_PROPERTY_PREFIX: &str = "--";

/// The unprefixed property name `node` references, if it is such a reference.
///
/// Answers with the name rather than with a yes, so the rejection can quote it:
/// a value may carry several references and "Unprefixed custom properties" on
/// its own leaves the author to find which one.
///
/// Reads one level: the reference's own first argument. A word there is the
/// property name — `var(--foo, red)` names `--foo`, `var(foo)` names `foo`.
/// Anything that is not a word is not a name at all (a nested `var()`, a
/// string, a comment) and is left alone, so a reference whose first argument
/// the author wrote as something else is never reported as a missing prefix.
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

/// Rejects a custom-property reference whose property name lacks its `--`
/// prefix.
///
/// Runs after the two unclosed detectors and before anything rewrites the
/// token list. Both halves of that placement are load-bearing. Running after
/// them means `var(foo` — unprefixed *and* unfinished — is reported as the
/// unfinished function it is, which is the more useful of the two things to say
/// and what this compiler has always said about it. Running before the
/// rewrites means the name quoted in the rejection is the name the author
/// typed, rather than one a later pass has already re-spelled.
///
/// Only top-level references are inspected, matching what this rule has always
/// checked: a `var()` nested inside `calc()` or a colour function is not
/// reached. Widening the walk would reject programs that compile today, which
/// is a decision about the rule rather than about where it reads from.
///
/// Rejects through [`super::reject_value`], like the two unclosed detectors, so
/// the rule text is quoted the same way in all three. The name is carried in
/// the message rather than instead of the rule: the name says which of several
/// references is wrong, the rule says which declaration they were in, and an
/// author looking at a large `create()` call needs both.
pub fn detect_unprefixed_custom_properties(ast: &mut ValueParser, key: &str) {
  if let Some(name) = ast.nodes.iter().find_map(unprefixed_property_name) {
    let message = format!("{UNPREFIXED_CUSTOM_PROPERTIES}: var({name})");

    reject_value(ast, key, &message);
  }
}

#[cfg(test)]
#[path = "../../tests/unprefixed_custom_properties_predicate_tests.rs"]
mod tests;
