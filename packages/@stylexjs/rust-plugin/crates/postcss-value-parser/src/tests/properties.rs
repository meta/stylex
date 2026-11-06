//! Invariants that must hold for every value: no crashes, well-formed offsets, walk order.

use crate::{
  Node, NodeKind, ValueParser, parse, stringify, stringify_node, stringify_node_with,
  stringify_with, unit,
};

use super::dump::dump;

/// The characters the scanner branches on, plus one letter and one digit.
const ALPHABET: &[char] = &['(', ')', '\'', '"', '\\', '/', '*', ',', ':', ' ', 'a', '0'];

/// Every string of `length` characters drawn from [`ALPHABET`].
fn combinations(length: u32) -> Vec<String> {
  let mut out = vec![String::new()];

  for _ in 0..length {
    let mut next = Vec::with_capacity(out.len() * ALPHABET.len());
    for prefix in &out {
      for ch in ALPHABET {
        let mut candidate = prefix.clone();
        candidate.push(*ch);
        next.push(candidate);
      }
    }
    out = next;
  }

  out
}

/// Every node in a tree, parents before children.
fn flatten<'nodes>(nodes: &'nodes [Node], out: &mut Vec<&'nodes Node>) {
  for node in nodes {
    out.push(node);
    if let Some(children) = node.nodes.as_deref() {
      flatten(children, out);
    }
  }
}

fn all_nodes(nodes: &[Node]) -> Vec<&Node> {
  let mut out = Vec::new();
  flatten(nodes, &mut out);
  out
}

#[test]
fn no_arrangement_of_the_characters_it_branches_on_can_crash_it() {
  for input in combinations(4) {
    let nodes = parse(&input);
    let _ = stringify(&nodes);
    let _ = dump(&nodes);
  }
}

/// Control, printable ASCII and non-ASCII characters outside [`ALPHABET`].
#[test]
fn no_short_input_of_any_character_can_crash_it() {
  let mut inputs: Vec<String> = (0u8..=0x7f)
    .map(|byte| char::from(byte).to_string())
    .collect();

  for left in 0u8..=0x7f {
    for right in 0u8..=0x7f {
      inputs.push(format!("{}{}", char::from(left), char::from(right)));
    }
  }

  for exotic in [
    "\u{0}",
    "\u{7f}",
    "\u{85}",
    "\u{a0}",
    "\u{feff}",
    "é",
    "→",
    "日本語",
    "🙂",
    "🙂\u{200d}🙂",
    "\u{301}",
    "\u{fffd}",
    "\\\u{0}",
    "\"🙂",
    "url(🙂",
    "/*🙂",
  ] {
    inputs.push(exotic.to_owned());
  }

  for input in inputs {
    let nodes = parse(&input);
    let _ = stringify(&nodes);
    let _ = dump(&nodes);
  }
}

#[test]
fn every_span_runs_forward_and_ends_within_a_byte_of_the_input() {
  for input in combinations(3) {
    for node in all_nodes(&parse(&input)) {
      assert!(
        node.source_index <= node.source_end_index,
        "span runs backwards for {input:?}: {node:?}"
      );
      assert!(
        node.source_end_index <= input.len() + 1,
        "span ends more than a byte past the input for {input:?}: {node:?}"
      );
    }
  }
}

#[test]
fn only_an_invented_quote_and_a_trailing_backslash_reach_past_the_input() {
  let overshooting: Vec<String> = combinations(3)
    .into_iter()
    .filter(|input| {
      all_nodes(&parse(input))
        .iter()
        .any(|node| node.source_end_index > input.len())
    })
    .collect();

  for input in &overshooting {
    assert!(
      // An unclosed string: an odd number of quotes, none of them escaped.
      input.ends_with('\\') || input.contains('\'') || input.contains('"'),
      "{input:?} overshoots for a third reason"
    );
  }

  assert_eq!(
    parse("(('")
      .first()
      .map(|node| (node.source_index, node.source_end_index)),
    Some((0, 4)),
    "the invented closing quote no longer extends the outer function's span"
  );
  assert_eq!(
    parse("a\\")
      .first()
      .map(|node| (node.source_index, node.source_end_index)),
    Some((0, 3)),
    "the trailing backslash no longer overshoots"
  );
}

/// Children start no earlier than their function (ends may overshoot on a trailing backslash).
#[test]
fn every_function_starts_no_later_than_its_children() {
  fn check(input: &str, nodes: &[Node], enclosing: Option<&Node>) {
    for node in nodes {
      if let Some(parent) = enclosing {
        assert!(
          parent.source_index <= node.source_index,
          "{:?} starts before its parent in {input:?}: {node:?} vs {parent:?}",
          node.value
        );
      }
      if let Some(children) = node.nodes.as_deref() {
        check(input, children, Some(node));
      }
    }
  }

  for input in combinations(4) {
    check(&input, &parse(&input), None);
  }
}

#[test]
fn offsets_count_bytes_through_multi_byte_characters() {
  let nodes = parse("🙂 x");

  assert_eq!(
    dump(&nodes),
    ["word \"🙂\" 0..4", "space \" \" 4..5", "word \"x\" 5..6"].join("\n")
  );
}

#[test]
fn an_empty_value_parses_to_no_nodes() {
  assert!(parse("").is_empty());
  assert_eq!(stringify(&[]), "");
  assert_eq!(ValueParser::new("").to_string(), "");
}

#[test]
fn deeply_nested_functions_parse_serialize_and_walk() {
  let depth = 512;
  let input = format!("{}1px{}", "calc(".repeat(depth), ")".repeat(depth));

  let mut parsed = ValueParser::new(&input);
  assert_eq!(parsed.to_string(), input);

  let mut visited = 0;
  parsed.walk(
    |_, _| {
      visited += 1;
      true
    },
    false,
  );
  // One function per level, plus the `1px` at the bottom.
  assert_eq!(visited, depth + 1);
}

#[test]
fn deeply_nested_unclosed_functions_are_all_flagged() {
  let depth = 512;
  let input = format!("{}1px", "calc(".repeat(depth));

  let mut parsed = ValueParser::new(&input);
  assert_eq!(parsed.to_string(), input);

  let mut functions = 0;
  parsed.walk(
    |node, _| {
      if node.kind == NodeKind::Function {
        functions += 1;
        assert!(node.unclosed, "level {functions} is not flagged unclosed");
        assert_eq!(node.source_end_index, input.len());
      }
      true
    },
    false,
  );
  assert_eq!(functions, depth);
}

/// Long values, to catch quadratic scanning or fixed-size buffers.
#[test]
fn very_long_values_are_scanned_whole() {
  for input in [
    "a".repeat(100_000),
    format!("\"{}\"", "a".repeat(100_000)),
    format!("/*{}*/", "a".repeat(100_000)),
    "a,".repeat(50_000),
    " ".repeat(100_000),
    "\\a".repeat(50_000),
  ] {
    assert_eq!(stringify(&parse(&input)), input);
  }
}

#[test]
fn a_walk_visits_nested_nodes_with_sibling_indices() {
  let mut parsed = ValueParser::new("a calc(1px + 2px)");
  let mut seen = Vec::new();

  parsed.walk(
    |node, index| {
      seen.push(format!("{}:{index}:{}", node.kind, node.value));
      true
    },
    false,
  );

  assert_eq!(
    seen,
    [
      "word:0:a",
      "space:1: ",
      "function:2:calc",
      "word:0:1px",
      "space:1: ",
      "word:2:+",
      "space:3: ",
      "word:4:2px",
    ]
  );
}

#[test]
fn a_walk_that_declines_a_function_does_not_enter_it() {
  let mut parsed = ValueParser::new("calc(1px) 2px");
  let mut seen = Vec::new();

  parsed.walk(
    |node, _| {
      seen.push(node.value.clone());
      node.kind != NodeKind::Function
    },
    false,
  );

  assert_eq!(seen, ["calc", " ", "2px"]);
}

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

/// Descent is decided by the kind after the callback, so re-kinding a function skips it.
#[test]
fn re_kinding_a_function_mid_walk_stops_the_walk_entering_it() {
  let mut parsed = ValueParser::new("fn( ) fn2( fn3())");
  let mut seen = Vec::new();

  parsed.walk(
    |node, _| {
      if node.kind == NodeKind::Function && node.value == "fn2" {
        node.kind = NodeKind::Word;
      }
      seen.push(format!("{}:{}", node.kind, node.value));
      true
    },
    false,
  );

  // `fn3` never appears: it is still inside the node now calling itself a word.
  assert_eq!(seen, ["function:fn", "space: ", "word:fn2"]);
}

/// The two values are the ones upstream's test uses.
#[test]
fn a_walk_that_does_nothing_changes_nothing() {
  for input in [
    " rgba( 34 , 45 , 54, .5 ) ",
    "w1 w2 w6 \n f(4) ( ) () \t \"s't\" 'st\\\"2'",
  ] {
    let mut parsed = ValueParser::new(input);

    assert_eq!(parsed.walk(|_, _| true, false).to_string(), input);
    assert_eq!(parsed.walk(|_, _| true, true).to_string(), input);
  }
}

#[test]
fn a_walk_can_rewrite_the_tree_it_is_walking() {
  let mut parsed = ValueParser::new("calc(500ms) 500ms");

  parsed.walk(
    |node, _| {
      if node.kind == NodeKind::Word && node.value == "500ms" {
        node.value = String::from(".5s");
      }
      true
    },
    false,
  );

  assert_eq!(parsed.to_string(), "calc(.5s) .5s");
}

/// Lists are restructured outside the walk (e.g. dropping the space before `!important`).
#[test]
fn a_node_list_is_restructured_outside_the_walk() {
  let mut parsed = ValueParser::new("1px !important");

  let important = parsed
    .nodes
    .iter()
    .position(|node| node.kind == NodeKind::Word && node.value == "!important");

  match important {
    Some(index) if index > 0 => {
      if matches!(parsed.nodes.get(index - 1), Some(node) if node.kind == NodeKind::Space) {
        parsed.nodes.remove(index - 1);
      }
    },
    _ => panic!("the annotation was not found where the walk would leave it"),
  }

  assert_eq!(parsed.to_string(), "1px!important");
}

#[test]
fn a_string_keeps_its_own_quote_and_its_own_bytes() {
  for (input, quote, contents) in [
    ("'sidebar content'", '\'', "sidebar content"),
    ("\"sidebar content\"", '"', "sidebar content"),
    ("'a \\' b'", '\'', "a \\' b"),
    ("\"a ' b\"", '"', "a ' b"),
    ("'a \" b'", '\'', "a \" b"),
    ("\"→ Привет 日本語 🙂\"", '"', "→ Привет 日本語 🙂"),
    ("\"\\201C\"", '"', "\\201C"),
    ("''", '\'', ""),
  ] {
    let nodes = parse(input);
    let node = match nodes.first() {
      Some(node) => node,
      None => panic!("{input:?} parsed to nothing"),
    };

    assert_eq!(node.kind, NodeKind::String);
    assert_eq!(node.quote, Some(quote), "quote changed for {input:?}");
    assert_eq!(node.value, contents, "contents changed for {input:?}");
    assert!(!node.unclosed);
    assert_eq!(stringify(&nodes), input);
  }
}

#[test]
fn what_ran_off_the_end_of_the_input_is_flagged_not_rejected() {
  for (input, kind) in [
    ("\"abc", NodeKind::String),
    ("'abc", NodeKind::String),
    ("/*abc", NodeKind::Comment),
    ("calc(1px", NodeKind::Function),
    ("url(abc", NodeKind::Function),
  ] {
    let nodes = parse(input);
    let node = match nodes.first() {
      Some(node) => node,
      None => panic!("{input:?} parsed to nothing"),
    };

    assert_eq!(node.kind, kind, "wrong kind for {input:?}");
    assert!(node.unclosed, "{input:?} is not flagged unclosed");
    assert_eq!(node.source_end_index, input.len());
    assert_eq!(stringify(&nodes), input);
  }
}

#[test]
fn a_url_body_is_one_word_however_much_css_syntax_it_contains() {
  for (input, body) in [
    (
      "url(https://example.com/a.png)",
      "https://example.com/a.png",
    ),
    (
      "url(data:image/svg+xml;base64,PHN2Zz48L3N2Zz4=)",
      "data:image/svg+xml;base64,PHN2Zz48L3N2Zz4=",
    ),
    ("url(a\\)b)", "a\\)b"),
    ("url(日本語.png)", "日本語.png"),
  ] {
    let nodes = parse(input);
    let node = match nodes.first() {
      Some(node) => node,
      None => panic!("{input:?} parsed to nothing"),
    };

    assert_eq!(node.kind, NodeKind::Function);
    assert_eq!(node.value, "url");
    assert_eq!(
      node.nodes.as_deref().map(|children| children
        .iter()
        .map(|child| child.value.as_str())
        .collect::<Vec<_>>()),
      Some(vec![body]),
      "url body split up for {input:?}"
    );
    assert_eq!(stringify(&nodes), input);
  }
}

#[test]
fn a_quoted_url_body_stays_a_string() {
  let nodes = parse("url('a,b.png')");

  assert_eq!(
    dump(&nodes),
    [
      "function \"url\" 0..14 before=\"\" after=\"\" nodes=1",
      "  string \"a,b.png\" 4..13 quote=\"'\"",
    ]
    .join("\n")
  );
}

#[test]
fn a_word_that_does_not_start_with_a_number_has_no_split() {
  for input in [
    "auto", "", " ", ".", "-", "+", "e3", "--custom", "#ffffff", "px",
  ] {
    assert_eq!(unit(input), None, "{input:?} split when it should not have");
  }
}

#[test]
fn a_unicode_range_is_not_a_word_followed_by_a_signed_number() {
  for input in ["U+26", "u+26", "U+0-7F", "U+4??"] {
    let nodes = parse(input);

    assert_eq!(
      nodes.first().map(|node| node.kind),
      Some(NodeKind::UnicodeRange),
      "{input:?} was not read as a range"
    );
    assert_eq!(nodes.len(), 1);
  }

  for input in ["U+", "U+zz", "Ux26", "u"] {
    assert_eq!(
      parse(input).first().map(|node| node.kind),
      Some(NodeKind::Word),
      "{input:?} was read as a range"
    );
  }
}

/// A function without a child list (only buildable by hand) spells as its bare name.
#[test]
fn a_function_with_no_child_list_serializes_as_its_name() {
  let node = Node::new(NodeKind::Function, String::from("calc"), 0, 4);

  assert_eq!(stringify_node(&node), "calc");
  assert_eq!(stringify(std::slice::from_ref(&node)), "calc");
}

/// A string node with no quote (only buildable by hand) spells as its contents.
#[test]
fn a_string_with_no_quote_serializes_as_its_contents() {
  let node = Node::new(NodeKind::String, String::from("abc"), 0, 3);

  assert_eq!(stringify_node(&node), "abc");
}

#[test]
fn one_node_and_a_list_of_it_spell_out_the_same() {
  for input in [
    "calc(1px + 2px)",
    "\"abc\"",
    "/*x*/",
    "U+26",
    " ",
    "1px",
    "url(a)",
  ] {
    let nodes = parse(input);

    let joined: String = nodes.iter().map(stringify_node).collect();

    assert_eq!(
      joined,
      stringify(&nodes),
      "the two spellings differ for {input:?}"
    );
  }
}

#[test]
fn an_override_reaches_nested_nodes_and_may_decline() {
  let nodes = parse("calc(1px + var(--bar))");

  assert_eq!(
    stringify_with(&nodes, &mut |_| None),
    "calc(1px + var(--bar))"
  );
  assert_eq!(
    stringify_with(&nodes, &mut |node: &Node| match node.value == "1px" {
      true => Some(String::from("2px")),
      false => None,
    }),
    "calc(2px + var(--bar))"
  );

  let function = match nodes.first() {
    Some(node) => node,
    None => panic!("the value parsed to nothing"),
  };
  assert_eq!(
    stringify_node_with(
      function,
      &mut |node: &Node| match node.kind == NodeKind::Function {
        true => Some(node.value.clone()),
        false => None,
      }
    ),
    "calc"
  );
}
