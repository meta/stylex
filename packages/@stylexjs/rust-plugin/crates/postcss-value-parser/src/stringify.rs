//! Serializing a node list back to text.

use crate::{Node, NodeKind};

/// A per-node override: `Some` replaces the node's text (children included), `None` declines.
pub type Custom<'call> = &'call mut dyn FnMut(&Node) -> Option<String>;

type MaybeCustom<'outer, 'call> = &'outer mut Option<Custom<'call>>;

/// Spells one node into `out`.
/// Siblings reach the override left to right; upstream visits them right to left.
fn write_node(node: &Node, custom: MaybeCustom<'_, '_>, out: &mut String) {
  if let Some(over) = custom.as_mut()
    && let Some(replacement) = over(node)
  {
    out.push_str(&replacement);
    return;
  }

  match node.kind {
    NodeKind::Word | NodeKind::Space => out.push_str(&node.value),
    NodeKind::String => {
      if let Some(quote) = node.quote {
        out.push(quote);
      }
      out.push_str(&node.value);
      if let Some(quote) = node.quote
        && !node.unclosed
      {
        out.push(quote);
      }
    },
    NodeKind::Comment => {
      out.push_str("/*");
      out.push_str(&node.value);
      if !node.unclosed {
        out.push_str("*/");
      }
    },
    NodeKind::Div => {
      out.push_str(node.before.as_deref().unwrap_or_default());
      out.push_str(&node.value);
      out.push_str(node.after.as_deref().unwrap_or_default());
    },
    // Matches upstream: a non-function node with children spells out as the children alone.
    NodeKind::UnicodeRange => match node.nodes.as_deref() {
      None => out.push_str(&node.value),
      Some(children) => write_list(children, custom, out),
    },
    // Matches upstream: a function without a child list spells out as its bare name.
    NodeKind::Function => match node.nodes.as_deref() {
      None => out.push_str(&node.value),
      Some(children) => {
        out.push_str(&node.value);
        out.push('(');
        out.push_str(node.before.as_deref().unwrap_or_default());
        write_list(children, custom, out);
        out.push_str(node.after.as_deref().unwrap_or_default());
        if !node.unclosed {
          out.push(')');
        }
      },
    },
  }
}

fn write_list(nodes: &[Node], custom: MaybeCustom<'_, '_>, out: &mut String) {
  for node in nodes {
    write_node(node, custom, out);
  }
}

/// Capacity hint from the span the nodes were parsed from; only a hint.
fn estimated_len(nodes: &[Node]) -> usize {
  match (nodes.first(), nodes.last()) {
    (Some(first), Some(last)) => last.source_end_index.saturating_sub(first.source_index),
    _ => 0,
  }
}

/// Spells a node list back out, in order.
pub fn stringify(nodes: &[Node]) -> String {
  let mut out = String::with_capacity(estimated_len(nodes));
  write_list(nodes, &mut None, &mut out);
  out
}

/// Spells one node back out; unclosed nodes stay unclosed.
/// The kind decides the spelling: a re-kinded function spells as its new kind.
pub fn stringify_node(node: &Node) -> String {
  let mut out = String::with_capacity(estimated_len(std::slice::from_ref(node)));
  write_node(node, &mut None, &mut out);
  out
}

/// [`stringify`], with an override consulted for every node it reaches.
pub fn stringify_with(nodes: &[Node], custom: Custom<'_>) -> String {
  let mut out = String::with_capacity(estimated_len(nodes));
  write_list(nodes, &mut Some(custom), &mut out);
  out
}

/// [`stringify_node`], with an override consulted for that node and its descendants.
pub fn stringify_node_with(node: &Node, custom: Custom<'_>) -> String {
  let mut out = String::with_capacity(estimated_len(std::slice::from_ref(node)));
  write_node(node, &mut Some(custom), &mut out);
  out
}
