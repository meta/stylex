//! Visiting every node. See the crate documentation in `lib.rs` for what it is
//! and who holds its copyright.

use crate::{Node, NodeKind};

/// Visits every node in `nodes`, descending into functions.
///
/// `cb` receives each node and its index among its siblings, and returns
/// whether to descend into it. Only a function node has children, so the answer
/// is ignored everywhere else.
///
/// With `bubble` set, children are visited before their parent — and the
/// callback's answer is ignored entirely, because the JavaScript never reads
/// it on that path. Descent is unconditional there.
///
/// That is worth stating because the JavaScript's own type declaration says the
/// opposite: that returning `false` prevents traversal *only* when `bubble` is
/// set. Its implementation reads the answer only when `bubble` is unset, and
/// its own test for refusing a function runs with `bubble` unset and does
/// refuse. The code and the test agree with each other; the prose is wrong, and
/// this follows the code.
///
/// # What a callback cannot do here
///
/// The JavaScript hands its callback the sibling array too, so a callback can
/// remove a sibling mid-walk — and one of the normalizers does, dropping the
/// space before an `!important`. Rust cannot lend out the node and the list
/// holding it at once, so this lends the node and the list stays the caller's.
///
/// The consequence is worth stating plainly, because the alternative is finding
/// it out later: a structural edit to a node list has to happen outside the
/// walk, before it or after it. That is already where two of the three edits in
/// question live.
///
/// The callback is taken as a trait object rather than by generic parameter.
/// Nine normalizers walking the same tree would otherwise be nine copies of
/// this function, and the recursion means the copies are not free; an indirect
/// call per node costs nothing next to the allocation each visited node's value
/// already carries.
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
