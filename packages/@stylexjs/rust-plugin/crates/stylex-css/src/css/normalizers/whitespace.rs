//! Ported normalizer 3 of 9 in upstream's sequence, which is what that count
//! names. See `normalize_value.rs` for the order the passes run in here — a
//! tenth pass that upstream has no equivalent for runs among them.

use postcss_value_parser::{Node, NodeKind, ValueParser};
use stylex_constants::constants::messages::{LINT_IMPORTANT_NOT_LAST, LINT_VALUE_HAS_NO_TOKENS};
use stylex_macros::stylex_panic;

/// The importance annotation, which the scanner hands over as one word token.
const IMPORTANT: &str = "!important";

/// Collapses every run of whitespace to a single space and fixes the spacing
/// around separators and function parentheses.
///
/// Spaces are rewritten in place rather than moved: a space between two words
/// stays between those two words, and the only node this ever removes is the
/// one before an importance annotation. That is what keeps a value the author
/// spelled unusually from being re-spelled.
pub fn normalize_whitespace(ast: &mut ValueParser, _key: &str) {
  trim_edges(&mut ast.nodes);

  // Planned against the untouched list before anything is rewritten, because
  // whether the reference implementation survives this normalizer at all is
  // decided by what its `!important` handler does to the list it is walking.
  //
  // Asked only of values that carry an annotation, which is very few of them.
  // Without an annotation the plan walk can only answer `Remove(vec![])` --
  // `Overrun` needs a removal and a removal needs an annotation -- and it pays
  // a `Vec<NodeKind>` as long as the value to get there.
  let removals = match carries_importance(&ast.nodes) {
    true => match plan_important_removals(&ast.nodes) {
      ImportantPlan::Remove(removals) => removals,
      ImportantPlan::Overrun => stylex_panic!("{}", LINT_IMPORTANT_NOT_LAST),
    },
    false => Vec::new(),
  };

  ast.walk(
    |node, _| {
      // Each arm checks before it writes. Most values arrive already spelled
      // the way this pass would spell them, and an unconditional assignment
      // allocates a replacement for text identical to what it replaces.
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
/// The reference implementation reads the first and last elements without
/// guarding, so a value that scans to no tokens at all — an empty string, or
/// one that is nothing but characters the scanner reads as whitespace — throws
/// a `TypeError` here.
///
/// [`LINT_VALUE_HAS_NO_TOKENS`] *replaces* that `TypeError` rather than
/// reproducing a failure the production seam can reach: `transform_value`
/// short-circuits a blank value before normalization is entered at all, so
/// nothing outside this crate's own tests arrives here with an empty list. The
/// rejection stays because the alternative is a panic from indexing, and a
/// named message is the better thing to fail with if that ever stops being
/// true.
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

/// What the reference implementation's walk does to the top-level list when it
/// meets an importance annotation.
enum ImportantPlan {
  /// Top-level indices to remove, in the order they are removed. Empty for
  /// every value that carries no importance annotation, which is nearly all of
  /// them.
  Remove(Vec<usize>),
  /// The walk read past the end of the list it had already shortened.
  Overrun,
}

/// Works out which top-level nodes the importance handler removes, and whether
/// the walk survives having done so.
///
/// Three details of the original are load-bearing here, and none of them is an
/// accident this compiler gets to correct — each one moves bytes into the
/// class-name hash or decides whether a value compiles at all.
///
/// The walk reads the list's length once, before it starts. Removing an element
/// shortens the list without shortening the walk, so every iteration still to
/// come now ends one index past the list — and reading that index is where the
/// reference implementation crashes. Only a removal during the final iteration
/// escapes it, which is why `red !important` normalizes and
/// `red !important blue` does not.
///
/// [`TopLevelList`] carries the other two: which list a removal reads and edits,
/// and when it declines to remove anything at all.
fn plan_important_removals(nodes: &[Node]) -> ImportantPlan {
  // Read once, before the walk starts, exactly as the original reads it. Every
  // overrun below is the gap between this number and the list's real length.
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

/// The top-level node list as the importance handler has left it, and what it
/// took out.
///
/// The two travel together because neither answers anything alone: the kinds
/// are what the next `nodes[idx - 1]` test reads, and they are only correct
/// once every removal so far has been applied to them.
struct TopLevelList {
  /// The kind of each surviving top-level node, shortened by every removal.
  /// Only kinds, because the handler only ever asks whether a node is a space.
  kinds: Vec<NodeKind>,
  /// The indices removed, in the order they were removed, each one an index
  /// into the list as it stood at the time.
  taken: Vec<usize>,
}

impl TopLevelList {
  fn of(nodes: &[Node]) -> Self {
    TopLevelList {
      kinds: nodes.iter().map(|node| node.kind).collect(),
      taken: Vec::new(),
    }
  }

  /// Removes the node at `index - 1` if it is a space, and reports nothing
  /// either way — the handler it models does not look.
  ///
  /// `index` is the annotation's index among *its own* siblings while the list
  /// is always the top-level one, which is the quirk that makes an annotation
  /// inside a function remove an unrelated node.
  fn take_space_before(&mut self, index: usize) {
    if index > 0 && self.kinds.get(index - 1) == Some(&NodeKind::Space) {
      self.taken.push(index - 1);
      self.kinds.remove(index - 1);
    }
  }

  fn lost_a_node(&self) -> bool {
    !self.taken.is_empty()
  }

  /// The removals, once the walk that decided them is over. Consumes the list,
  /// because the kinds are only meaningful mid-walk and nothing downstream
  /// should be tempted to read them afterwards.
  fn into_removals(self) -> Vec<usize> {
    self.taken
  }
}

/// Whether a node is the importance annotation the plan walk acts on.
///
/// Shared with [`carries_importance`] rather than spelled twice, because a
/// pre-check looser than this predicate would plan for values the walk ignores,
/// and one tighter would skip planning for values it does not.
fn names_importance(node: &Node) -> bool {
  node.kind == NodeKind::Word && node.value == IMPORTANT
}

/// Whether an annotation appears anywhere the plan walk would reach it.
///
/// Descends exactly where [`plan_node`] descends — into functions, and only
/// into functions — so the two agree about which values have nothing to plan.
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
