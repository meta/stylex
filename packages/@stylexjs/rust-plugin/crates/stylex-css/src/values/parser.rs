//! Splits a shorthand value into the parts its expansion hands out.
//!
//! Ports `splitValue` from upstream `shared/utils/split-css-value.js`, cutting on the
//! [value scanner]'s node kinds so `10px/1.5` is two parts but `calc(100% / 3)` is one.
//! Parts are echoed verbatim, a trailing `!important` is applied to every part, and an empty
//! part (e.g. from `padding: '1px /*'`) is kept; upstream throws on that input.
//!
//! [value scanner]: postcss_value_parser

use crate::css::common::nests_too_deeply;
use postcss_value_parser::{Node, NodeKind, ValueParser};

/// The parts of `css_string`, in the order the expansion consumes them; empty parts are kept.
pub fn split_value_parts(css_string: &str) -> Vec<String> {
  let trimmed = js_trim(css_string);

  // Nested past the depth budget: hand the value on whole so normalization raises the
  // nesting-depth diagnostic (scanning it would overflow the stack in the tree's destructor).
  if trimmed.contains('(') && nests_too_deeply(trimmed) {
    return vec![trimmed.to_string()];
  }

  let parts: Vec<String> = ValueParser::new(trimmed)
    .nodes
    .iter()
    .filter(|node| !matches!(node.kind, NodeKind::Space | NodeKind::Div))
    .map(print_node)
    .collect();

  apply_importance(parts)
}

/// Moves a trailing `!important` onto every other part; a lone `!important` is left alone.
fn apply_importance(mut parts: Vec<String>) -> Vec<String> {
  // No character lowercases into `!`, so the prefix test keeps the case fold off the common path.
  let qualifies = parts.len() >= 2
    && parts
      .last()
      .is_some_and(|last| last.starts_with('!') && last.to_lowercase() == "!important");

  if !qualifies {
    return parts;
  }

  parts.pop();

  for part in &mut parts {
    part.push_str(" !important");
  }

  parts
}

/// One node as the text it contributes to its part.
///
/// Iterative, since this runs before the depth guard and input may nest arbitrarily deep.
fn print_node(node: &Node) -> String {
  let mut out = String::with_capacity(node.source_end_index.saturating_sub(node.source_index));
  let mut pending: Vec<Emit<'_>> = vec![Emit::Node(node)];

  while let Some(step) = pending.pop() {
    match step {
      Emit::CloseParen => out.push(')'),
      Emit::Node(node) => match (node.kind, node.quote, node.nodes.as_deref()) {
        (NodeKind::String, Some(quote), _) => {
          out.push(quote);
          out.push_str(&node.value);
          out.push(quote);
        },
        (NodeKind::Function, _, Some(arguments)) => {
          out.push_str(&node.value);
          out.push('(');
          pending.push(Emit::CloseParen);
          pending.extend(arguments.iter().rev().map(Emit::Node));
        },
        // Matches upstream: a string or function missing the field above falls back to its value.
        _ => out.push_str(&node.value),
      },
    }
  }

  out
}

/// A step of [`print_node`]'s traversal: a node to spell, or the parenthesis
/// that closes the function whose arguments were just queued.
enum Emit<'a> {
  Node(&'a Node),
  CloseParen,
}

/// `css_string` trimmed like JavaScript's `String.prototype.trim` (BOM trimmed, `U+0085` kept).
fn js_trim(css_string: &str) -> &str {
  css_string.trim_matches(|character: char| {
    character == '\u{feff}' || (character.is_whitespace() && character != '\u{85}')
  })
}

#[cfg(test)]
#[path = "../tests/values_parser_tests.rs"]
mod tests;
