//! Visiting every node.

use crate::{Node, NodeKind};

/// Visits every node, descending into functions; `cb` gets the node and its sibling index and
/// returns whether to descend. With `bubble`, children come first and the answer is ignored.
///
/// Unlike upstream, `cb` cannot see the sibling list, so structural edits must happen outside
/// the walk.
pub fn walk(nodes: &mut [Node], cb: &mut dyn FnMut(&mut Node, usize) -> bool, bubble: bool) {
  for (index, node) in nodes.iter_mut().enumerate() {
    let descend = match bubble {
      true => true,
      false => cb(node, index),
    };

    if descend
      && node.kind == NodeKind::Function
      && let Some(children) = node.nodes.as_mut()
    {
      walk(children, cb, bubble);
    }

    if bubble {
      cb(node, index);
    }
  }
}
