//! The unprefixed-custom-property predicate, on hand-built node shapes the scanner never
//! produces. Behaviour is covered in `css/tests/unprefixed_custom_properties_test.rs`.

use postcss_value_parser::{Node, NodeKind, ValueParser};

use super::{detect_unprefixed_custom_properties, unprefixed_property_name};

fn node(kind: NodeKind, value: &str) -> Node {
  Node::new(kind, value.to_string(), 0, value.len())
}

fn function(name: &str, args: Vec<Node>) -> Node {
  let mut func = node(NodeKind::Function, name);
  func.nodes = Some(args);
  func
}

#[test]
fn a_reference_naming_a_bare_word_is_unprefixed() {
  assert_eq!(
    unprefixed_property_name(&function("var", vec![node(NodeKind::Word, "foo")])),
    Some("foo")
  );
}

#[test]
fn a_reference_naming_a_prefixed_word_is_not() {
  assert_eq!(
    unprefixed_property_name(&function("var", vec![node(NodeKind::Word, "--foo")])),
    None
  );
}

#[test]
fn a_single_leading_dash_is_unprefixed() {
  assert_eq!(
    unprefixed_property_name(&function("var", vec![node(NodeKind::Word, "-foo")])),
    Some("-foo")
  );
}

/// Unreachable via the scanner, which always gives a function an argument list.
#[test]
fn a_function_with_no_argument_list_names_nothing() {
  let mut func = node(NodeKind::Function, "var");
  func.nodes = None;

  assert_eq!(unprefixed_property_name(&func), None);
}

#[test]
fn a_function_with_an_empty_argument_list_names_nothing() {
  assert_eq!(unprefixed_property_name(&function("var", vec![])), None);
}

/// Unreachable via a value; reported, which is the safe direction.
#[test]
fn an_empty_first_word_is_unprefixed() {
  assert_eq!(
    unprefixed_property_name(&function("var", vec![node(NodeKind::Word, "")])),
    Some("")
  );
}

#[test]
fn a_first_argument_that_is_not_a_word_names_nothing() {
  let kinds = [
    NodeKind::String,
    NodeKind::Div,
    NodeKind::Space,
    NodeKind::Comment,
    NodeKind::UnicodeRange,
  ];

  for kind in kinds {
    assert_eq!(
      unprefixed_property_name(&function("var", vec![node(kind, "foo")])),
      None,
      "a first argument of kind `{}` should not be read as a property name",
      kind.as_str()
    );
  }
}

#[test]
fn a_nested_reference_as_the_first_argument_names_nothing() {
  let inner = function("var", vec![node(NodeKind::Word, "foo")]);

  assert_eq!(
    unprefixed_property_name(&function("var", vec![inner])),
    None
  );
}

#[test]
fn a_function_that_is_not_a_reference_is_never_reported() {
  for name in ["calc", "rgb", "url", "attr", "--var", "Var", "VAR", ""] {
    assert_eq!(
      unprefixed_property_name(&function(name, vec![node(NodeKind::Word, "foo")])),
      None,
      "`{name}()` should not be read as a custom-property reference"
    );
  }
}

#[test]
fn a_node_that_is_not_a_function_is_never_reported() {
  for kind in [
    NodeKind::Word,
    NodeKind::String,
    NodeKind::Div,
    NodeKind::Space,
    NodeKind::Comment,
    NodeKind::UnicodeRange,
  ] {
    assert_eq!(
      unprefixed_property_name(&node(kind, "var")),
      None,
      "a node of kind `{}` should not be read as a reference",
      kind.as_str()
    );
  }
}

/// Whether the pass accepted `value`, i.e. did not panic.
fn accepts(value: &str, key: &str) -> bool {
  let value = value.to_owned();
  let key = key.to_owned();

  std::panic::catch_unwind(move || {
    let mut ast = ValueParser::new(&value);
    detect_unprefixed_custom_properties(&mut ast, &key);
  })
  .is_ok()
}

#[test]
fn the_pass_reads_the_top_level_only() {
  assert!(accepts("calc(var(foo) + 1px)", "width"));
  // The same reference at the top level is rejected.
  assert!(!accepts("var(foo)", "width"));
}

#[test]
fn an_empty_token_list_is_accepted() {
  assert!(accepts("", "color"));
}

#[test]
fn the_rejection_quotes_the_offending_name() {
  let panicked = std::panic::catch_unwind(|| {
    let mut ast = ValueParser::new("var(--fine) var(broken)");
    detect_unprefixed_custom_properties(&mut ast, "color");
  });

  let message = match panicked {
    Ok(()) => panic!("`var(broken)` should have been rejected"),
    Err(payload) => match payload.downcast_ref::<String>() {
      Some(message) => message.clone(),
      None => panic!("the rejection payload was not a String"),
    },
  };

  assert!(
    message.contains("var(broken)"),
    "the rejection should quote the offending name, got: {message}"
  );
}
