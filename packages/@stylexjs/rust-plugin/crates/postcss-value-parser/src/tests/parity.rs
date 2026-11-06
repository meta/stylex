//! Does this agree with the JavaScript it stands in for?
//!
//! Every expectation here is that JavaScript's own answer, printed by
//! `scripts/generate-value-parser-cases.mjs` into `cases.rs`. Nothing in this
//! file is written by eye, and nothing in it should be: a hand-edited
//! expectation here is just a divergence with a test blessing it.
//!
//! Regenerate the table after adding an input to the generator:
//!
//! ```sh
//! pnpm run --filter=@stylexswc/postcss-value-parser generate:value-parser-cases
//! ```

use crate::{
  Dimension, Node, NodeKind, ValueParser, parse, stringify, stringify_node_with, stringify_with,
  unit,
};

use super::cases::{OVERRIDE_CASES, PARSER_CASES, STRESS_CASES, UNIT_CASES};
use super::dump::dump;

#[test]
fn parses_every_value_into_the_same_tree() {
  for case in PARSER_CASES {
    assert_eq!(
      dump(&parse(case.input)),
      case.ast,
      "tree differs for {:?}",
      case.input
    );
  }
}

#[test]
fn serializes_every_value_into_the_same_text() {
  for case in PARSER_CASES {
    assert_eq!(
      stringify(&parse(case.input)),
      case.output,
      "serialized text differs for {:?}",
      case.input
    );
  }
}

/// The property the rest of the normalization effort rests on: what no
/// normalizer rewrites comes back exactly as the author wrote it — hex
/// spelling, letter case, quote character, exponent and whitespace positions
/// included.
///
/// Four corpus values are excluded, and they are the exception that proves
/// the agreement is real rather than merely plausible. `/*/` closes the comment
/// it opens, so a value containing one comes back changed. Matching that is the
/// requirement; improving on it would change class names.
#[test]
fn round_trips_every_value_byte_for_byte() {
  let mut quirks = Vec::new();

  for case in PARSER_CASES {
    match case.output == case.input {
      true => assert_eq!(
        stringify(&parse(case.input)),
        case.input,
        "round trip differs for {:?}",
        case.input
      ),
      false => quirks.push(case.input),
    }
  }

  assert_eq!(
    quirks,
    vec!["/*/ x */ 1px", "1px /*/ y */", "/*/", "/*/ x */"]
  );
}

#[test]
fn survives_values_at_the_sizes_where_scanning_stops_being_about_css() {
  for case in STRESS_CASES {
    assert_eq!(
      stringify(&parse(case.input)),
      case.output,
      "serialized text differs for the {} case",
      case.label
    );
  }
}

#[test]
fn splits_every_word_into_the_same_number_and_unit() {
  for (input, expected) in UNIT_CASES {
    let expected = expected.map(|(number, unit)| Dimension { number, unit });

    assert_eq!(unit(input), expected, "unit split differs for {input:?}");
  }
}

/// The entry point ties the three together, so it is measured against the same
/// answers rather than trusted to call them correctly.
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

/// Collapses a function to `name[arg,arg,arg]`, ignoring everything else. The
/// twin of `bracketFunctions` in the generator.
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

/// Serialising with a per-node override, against the answers a real run gave.
///
/// Each scenario is written twice on purpose — once in the generator, once here
/// — because an override is behaviour rather than data. What is never written
/// twice is the answer.
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
      // No override at all: a node re-kinded after parsing spells out as its
      // kind says, dropping the children and parentheses it still carries.
      "function-retyped-as-word" => {
        match nodes.get_mut(1) {
          Some(node) => node.kind = NodeKind::Word,
          None => panic!("{:?} has no second node", case.input),
        }
        stringify(&nodes)
      },
      // And the kind that is not tested before the children are: it keeps them
      // and drops the name and parentheses instead. The two scenarios differ
      // only in the kind assigned, which is the whole point of having both.
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

/// The one thing about an override that is *not* parity: the order it is
/// consulted in.
///
/// The JavaScript builds its result by prepending each node's text to what it
/// has so far, so a stateful override sees the siblings right to left. This
/// crate writes into a single sink and therefore sees them left to right. The
/// text both produce is identical, which is why no in-repo caller — all of
/// them stateless — can tell. Pinned so that a future rewrite has to decide to
/// change it rather than change it by accident.
#[test]
fn an_override_is_consulted_left_to_right() {
  let nodes = parse("a b c");
  let mut seen = Vec::new();

  let text = stringify_with(&nodes, &mut |node: &Node| {
    seen.push(node.value.clone());
    None
  });

  assert_eq!(seen, ["a", " ", "b", " ", "c"]);
  assert_eq!(text, "a b c");
}

/// And a parent still precedes its children, which is the half that *does*
/// match the JavaScript.
#[test]
fn an_override_sees_a_function_before_its_arguments() {
  let nodes = parse("calc(1px + 2px)");
  let mut seen = Vec::new();

  stringify_with(&nodes, &mut |node: &Node| {
    seen.push(node.value.clone());
    None
  });

  match seen.first() {
    Some(first) => assert_eq!(first, "calc"),
    None => panic!("the override was never consulted"),
  }
  assert!(seen.contains(&String::from("1px")));
}
