//! Parity with the JavaScript: every expectation comes from
//! `scripts/generate-value-parser-cases.mjs` via `cases.rs`; never edit them by hand.
//!
//! Regenerate the table after adding an input to the generator:
//!
//! ```sh
//! pnpm run --filter=@stylexswc/postcss-value-parser generate:value-parser-cases
//! ```

use crate::{Node, NodeKind, ValueParser, parse, stringify, stringify_node_with, stringify_with};

use super::cases::{OVERRIDE_CASES, PARSER_CASES};
use super::dump::dump;

#[test]
fn the_entry_point_parses_and_serializes_like_its_parts() {
  for case in PARSER_CASES {
    let parsed = ValueParser::new(case.input);

    assert_eq!(
      dump(&parsed.nodes),
      case.ast,
      "tree differs for {:?}",
      case.input
    );
    assert_eq!(
      parsed.to_string(),
      case.output,
      "serialized text differs for {:?}",
      case.input
    );
  }
}

/// Collapses a function to `name[arg,arg,arg]`; twin of `bracketFunctions` in the generator.
fn bracket_functions(node: &Node) -> Option<String> {
  if node.kind != NodeKind::Function {
    return None;
  }

  let children = node.nodes.as_deref()?;
  let args: Vec<&str> = [0, 2, 4]
    .iter()
    .filter_map(|at| children.get(*at))
    .map(|child| child.value.as_str())
    .collect();

  Some(format!("{}[{}]", node.value, args.join(",")))
}

/// Serializing with a per-node override; scenarios mirror the generator's.
#[test]
fn an_override_replaces_the_same_nodes() {
  for case in OVERRIDE_CASES {
    let mut nodes = parse(case.input);

    let produced = match case.label {
      "function-to-bracket-list" => stringify_with(&nodes, &mut bracket_functions),
      "function-to-bracket-list-one-node" => match nodes.get(1) {
        Some(node) => stringify_node_with(node, &mut bracket_functions),
        None => panic!("{:?} has no second node", case.input),
      },
      "replace-nested-function" => stringify_with(&nodes, &mut |node: &Node| match node.kind
        == NodeKind::Function
        && node.value == "var"
      {
        true => Some(String::from("10px")),
        false => None,
      }),
      "override-declines-every-node" => stringify_with(&nodes, &mut |_: &Node| None),
      // A re-kinded node spells out as its new kind, dropping children and parentheses.
      "function-retyped-as-word" => {
        match nodes.get_mut(1) {
          Some(node) => node.kind = NodeKind::Word,
          None => panic!("{:?} has no second node", case.input),
        }
        stringify(&nodes)
      },
      // Unicode-range keeps the children and drops the name and parentheses.
      "function-retyped-as-unicode-range" => {
        match nodes.get_mut(1) {
          Some(node) => node.kind = NodeKind::UnicodeRange,
          None => panic!("{:?} has no second node", case.input),
        }
        stringify(&nodes)
      },
      other => panic!("no Rust side for the {other:?} scenario"),
    };

    assert_eq!(produced, case.output, "the {} scenario differs", case.label);
  }
}
