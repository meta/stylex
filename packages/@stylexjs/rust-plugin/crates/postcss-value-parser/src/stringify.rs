//! Spelling a node list back out. See the crate documentation in `lib.rs`
//! for what it is and who holds its copyright.

use crate::{Node, NodeKind};

/// A per-node override, consulted before anything else and free to decline.
///
/// Returning `Some` replaces that node's text outright, children and
/// delimiters included; returning `None` leaves it to be spelled normally. The
/// override reaches nested nodes too, so a function inside a function can be
/// replaced without the outer one knowing.
pub type Custom<'call> = &'call mut dyn FnMut(&Node) -> Option<String>;

/// The override, or its absence, threaded down the recursion. Held behind a
/// second `&mut` so that each level can reborrow it for the next.
type MaybeCustom<'outer, 'call> = &'outer mut Option<Custom<'call>>;

/// Spells one node into `out`.
///
/// A sink rather than a returned `String`, because the JavaScript's shape --
/// build each node's text, then concatenate -- costs one allocation per node
/// on a path every declaration value in a compiled file goes through. The text
/// produced is the same byte for byte.
///
/// The order the override is consulted in diverges from the JavaScript
/// deliberately. A parent is still visited before its children, but siblings
/// are visited left to right, where the JavaScript builds its string by
/// prepending from the tail and therefore visits them right to left. Only a
/// stateful override can tell the difference, and reproducing an accident of
/// string prepending would cost this design its single sink. Pinned by
/// `an_override_is_consulted_left_to_right` in `tests/parity.rs`.
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
    // The JavaScript reaches both of the branches below through one
    // `Array.isArray` test on `nodes`, and only then asks whether the node is a
    // function. A node carrying children that is *not* a function spells out as
    // those children alone -- no name, no parentheses. Parsing never builds one,
    // but re-kinding a function mid-walk does, which is the same way the
    // function-spelled-as-a-word case arises.
    NodeKind::UnicodeRange => match node.nodes.as_deref() {
      None => out.push_str(&node.value),
      Some(children) => write_list(children, custom, out),
    },
    // A function that somehow lost its child list fails that same
    // `Array.isArray` test and falls through to spelling out as its bare name.
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

/// A buffer big enough for what `nodes` will spell out to, guessed from the
/// span they were read from.
///
/// The first node's start to the last node's end is the length of the text they
/// were parsed out of, which is the answer exactly whenever nothing has been
/// rewritten and close to it when something has. Reading it costs two field
/// accesses rather than a walk, and it is only a hint: too small buys one
/// regrowth, too large one oversized buffer, and neither changes a byte of what
/// comes out.
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

/// Spells one node back out.
///
/// A node whose delimiter was never closed is emitted without a closing
/// delimiter, so an unclosed construct survives the round trip as it was
/// written rather than acquiring a terminator the author did not type.
///
/// The kind decides everything: a node whose kind says word spells out as its
/// value even if it is still carrying the children and parentheses of the
/// function it used to be. A kind that is neither word, space, string, comment,
/// div nor function is the exception -- it spells out as its children when it
/// has any.
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

/// [`stringify_node`], with an override consulted for that node and every node
/// beneath it.
pub fn stringify_node_with(node: &Node, custom: Custom<'_>) -> String {
  let mut out = String::with_capacity(estimated_len(std::slice::from_ref(node)));
  write_node(node, &mut Some(custom), &mut out);
  out
}
