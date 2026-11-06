//! Invariants that must hold for every value: no crashes, well-formed offsets, walk order.

use crate::{Node, NodeKind, ValueParser, stringify, stringify_node};

#[test]
fn a_bubbling_walk_visits_children_first_and_ignores_the_answer() {
  let mut parsed = ValueParser::new("fn2( fn3())");
  let mut seen = Vec::new();

  parsed.walk(
    |node, _| {
      if node.kind == NodeKind::Function {
        seen.push(node.value.clone());
      }
      false
    },
    true,
  );

  assert_eq!(seen, ["fn3", "fn2"]);
}

/// A function without a child list (only buildable by hand) spells as its bare name.
#[test]
fn a_function_with_no_child_list_serializes_as_its_name() {
  let node = Node::new(NodeKind::Function, String::from("calc"), 0, 4);

  assert_eq!(stringify_node(&node), "calc");
  assert_eq!(stringify(std::slice::from_ref(&node)), "calc");
}
