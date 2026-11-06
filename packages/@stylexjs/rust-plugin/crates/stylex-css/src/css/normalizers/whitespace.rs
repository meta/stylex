//! Ported normalizer 3 of 9 in upstream's order; `normalize_value.rs` has the run order.

use postcss_value_parser::{Node, NodeKind, ValueParser};
use stylex_constants::constants::messages::{LINT_IMPORTANT_NOT_LAST, LINT_VALUE_HAS_NO_TOKENS};
use stylex_macros::stylex_panic;

/// The importance annotation, which the scanner hands over as one word token.
const IMPORTANT: &str = "!important";

/// Collapses whitespace runs to one space and fixes spacing around separators and parentheses.
///
/// Spaces are rewritten in place; the only node removed is a space before `!important`.
pub fn normalize_whitespace(ast: &mut ValueParser, _key: &str) {
  trim_edges(&mut ast.nodes);

  // Planned on the untouched list, mirroring upstream's `!important` handler; only values that
  // carry an annotation need the plan.
  let removals = match carries_importance(&ast.nodes) {
    true => match plan_important_removals(&ast.nodes) {
      ImportantPlan::Remove(removals) => removals,
      ImportantPlan::Overrun => stylex_panic!("{}", LINT_IMPORTANT_NOT_LAST),
    },
    false => Vec::new(),
  };

  ast.walk(
    |node, _| {
      // Check before writing, to avoid allocating identical replacements.
      match node.kind {
        NodeKind::Space => {
          if node.value != " " {
            node.value = " ".to_owned();
          }
        },
        NodeKind::Div => {
          let padding = match node.value.as_str() {
            "," => "",
            _ => " ",
          };

          if node.before.as_deref() != Some(padding) {
            node.before = Some(padding.to_owned());
          }

          if node.after.as_deref() != Some(padding) {
            node.after = Some(padding.to_owned());
          }
        },
        NodeKind::Function => {
          node.before = Some(String::new());
          node.after = Some(String::new());
          if node.value.eq_ignore_ascii_case("url") {
            return false;
          }
        },
        _ => {},
      }

      true
    },
    false,
  );

  for index in removals {
    ast.nodes.remove(index);
  }
}

/// Drops a leading and a trailing space node.
///
/// Panics with [`LINT_VALUE_HAS_NO_TOKENS`] on an empty list, where upstream throws a `TypeError`.
fn trim_edges(nodes: &mut Vec<Node>) {
  match nodes.first() {
    Some(first) if first.kind == NodeKind::Space => {
      nodes.remove(0);
    },
    Some(_) => {},
    None => stylex_panic!("{}", LINT_VALUE_HAS_NO_TOKENS),
  }

  match nodes.last() {
    Some(last) if last.kind == NodeKind::Space => {
      nodes.pop();
    },
    Some(_) => {},
    None => stylex_panic!("{}", LINT_VALUE_HAS_NO_TOKENS),
  }
}

/// What upstream's walk does to the top-level list when it meets `!important`.
enum ImportantPlan {
  /// Top-level indices to remove, in removal order.
  Remove(Vec<usize>),
  /// The walk read past the end of the list it had already shortened.
  Overrun,
}

/// Which top-level nodes upstream's `!important` handler removes, and whether its walk survives.
///
/// Matches upstream: the walk length is read once, so a removal before the final iteration
/// overruns the list (`red !important` normalizes, `red !important blue` does not).
fn plan_important_removals(nodes: &[Node]) -> ImportantPlan {
  // Read once, as upstream does; an overrun is the gap between this and the real length.
  let walk_length = nodes.len();
  let mut top_level = TopLevelList::of(nodes);

  for (index, node) in nodes.iter().enumerate() {
    plan_node(node, index, &mut top_level);

    if top_level.lost_a_node() && index + 1 < walk_length {
      return ImportantPlan::Overrun;
    }
  }

  ImportantPlan::Remove(top_level.into_removals())
}

/// The top-level node kinds as the `!important` handler has left them, and what it removed.
struct TopLevelList {
  /// The kind of each surviving top-level node.
  kinds: Vec<NodeKind>,
  /// The indices removed, in order, each into the list as it stood then.
  taken: Vec<usize>,
}

impl TopLevelList {
  fn of(nodes: &[Node]) -> Self {
    TopLevelList {
      kinds: nodes.iter().map(|node| node.kind).collect(),
      taken: Vec::new(),
    }
  }

  /// Removes the node at `index - 1` if it is a space.
  ///
  /// Matches upstream: `index` is among the annotation's own siblings but the list is top-level.
  fn take_space_before(&mut self, index: usize) {
    if index > 0 && self.kinds.get(index - 1) == Some(&NodeKind::Space) {
      self.taken.push(index - 1);
      self.kinds.remove(index - 1);
    }
  }

  fn lost_a_node(&self) -> bool {
    !self.taken.is_empty()
  }

  /// The removals, once the walk is over.
  fn into_removals(self) -> Vec<usize> {
    self.taken
  }
}

/// Whether a node is the importance annotation; shared with [`carries_importance`].
fn names_importance(node: &Node) -> bool {
  node.kind == NodeKind::Word && node.value == IMPORTANT
}

/// Whether an annotation appears where [`plan_node`] would reach it (descending into functions).
fn carries_importance(nodes: &[Node]) -> bool {
  nodes.iter().any(|node| {
    if names_importance(node) {
      return true;
    }

    match node.kind == NodeKind::Function {
      true => node.nodes.as_deref().is_some_and(carries_importance),
      false => false,
    }
  })
}

/// One node of the plan walk, and its children when it has any.
fn plan_node(node: &Node, index: usize, top_level: &mut TopLevelList) {
  if names_importance(node) {
    top_level.take_space_before(index);
  }

  if node.kind == NodeKind::Function
    && let Some(children) = node.nodes.as_deref()
  {
    for (child_index, child) in children.iter().enumerate() {
      plan_node(child, child_index, top_level);
    }
  }
}
