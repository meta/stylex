//! Tests for the reader that records which style namespaces a bailed-out
//! `stylex.props` call still needs at runtime.

use std::rc::Rc;

use stylex_enums::style_vars_to_keep::NonNullProps;
use stylex_state::{functions::FunctionMap, state_manager::StateManager};
use swc_core::{atoms::Atom, ecma::ast::Expr};

use crate::shared::utils::core::member_expression::{declared_namespaces, member_expression};
use crate::tests::support::expr;

/// Reads `code` as one member expression and hands back what the reader
/// recorded.
///
/// The counters start where the walk starts them: at the first argument, with
/// no bail-out point behind it and no namespace named yet.
fn namespaces_of(code: &str) -> NonNullProps {
  read(code, NonNullProps::Vec(vec![]), None, 0)
}

fn read(
  code: &str,
  mut non_null_props: NonNullProps,
  mut bail_out_index: Option<i32>,
  mut index: i32,
) -> NonNullProps {
  let member = match expr(code) {
    Expr::Member(member) => member,
    other => panic!("the fixture {code} is not a member expression: {other:?}"),
  };

  let mut state = StateManager::default();

  member_expression(
    &member,
    &mut index,
    &mut bail_out_index,
    &mut non_null_props,
    &mut state,
    &Rc::new(FunctionMap::default()),
  );

  non_null_props
}

fn named(names: &[&str]) -> NonNullProps {
  NonNullProps::Vec(names.iter().map(|name| (*name).into()).collect())
}

/// Every property the namespace declares is a name the runtime still needs.
#[test]
fn names_every_property_the_namespace_declares() {
  assert_eq!(
    namespaces_of("({ root: { color: 'red', margin: '1px' } }).root"),
    named(&["color", "margin"])
  );
}

/// A property declared as absent declares nothing, so the runtime needs no name
/// for it.
#[test]
fn leaves_out_a_property_that_declares_nothing() {
  assert_eq!(
    namespaces_of("({ root: { color: 'red', margin: null } }).root"),
    named(&["color"])
  );
}

/// A key the printer has to quote names its property like any other. Read
/// through `as_ident` it answered nothing at all, so the namespace behind it
/// was missing from the keep list and the null sweep dropped a name the runtime
/// still reads. The reference lists the keys of the folded object, which names
/// a quoted key as readily as a bare one.
#[test]
fn names_a_property_whose_key_has_to_be_quoted() {
  assert_eq!(
    namespaces_of("({ root: { '--my-color': 'red', color: 'blue' } }).root"),
    named(&["--my-color", "color"])
  );
}

/// The same rule for the three other shapes a key is written in. A number key
/// names a string property spelled the way JavaScript spells the number, and a
/// computed key naming a literal is the key that literal spells.
#[test]
fn names_a_property_whose_key_is_not_written_as_a_name() {
  assert_eq!(
    namespaces_of("({ root: { 0: 'red', 1e21: 'blue', ['--x']: 'green' } }).root"),
    named(&["0", "1e+21", "--x"])
  );
}

/// A quoted key that declares nothing is still left out, so the reader is
/// narrowing on the value and not on how the key was written.
#[test]
fn leaves_out_a_quoted_key_that_declares_nothing() {
  assert_eq!(
    namespaces_of("({ root: { '--my-color': null, color: 'blue' } }).root"),
    named(&["color"])
  );
}

#[test]
fn names_nothing_of_an_empty_namespace() {
  assert_eq!(namespaces_of("({ root: {} }).root"), named(&[]));
}

/// A reader that already needs every namespace has nothing left to narrow, so
/// it reads no further.
#[test]
fn a_reader_that_needs_everything_reads_no_further() {
  assert_eq!(
    read(
      "({ root: { color: 'red' } }).root",
      NonNullProps::True,
      None,
      0
    ),
    NonNullProps::True
  );
}

/// Past the point the walk bailed out at, the call can hold anything, so every
/// namespace is needed.
#[test]
fn everything_past_the_bail_out_point_is_needed() {
  assert_eq!(
    read("({ root: { color: 'red' } }).root", named(&[]), Some(0), 1),
    NonNullProps::True
  );
}

/// An expression that does not fold tells the reader nothing about which
/// namespaces are needed, so every one of them is.
#[test]
fn an_expression_that_does_not_fold_needs_everything() {
  assert_eq!(namespaces_of("unknown.root"), NonNullProps::True);
}

/// A spread, a key that is not a name, and a property with no value of its own
/// never reach the reader: the evaluator merges the first away and refuses the
/// other two, so a namespace that holds one folds to nothing at all.
#[test]
fn a_namespace_the_evaluator_cannot_fold_needs_everything() {
  assert_eq!(
    namespaces_of("({ root: { method() { return 1 } } }).root"),
    NonNullProps::True
  );
  assert_eq!(
    namespaces_of("({ root: { get color() { return 1 } } }).root"),
    NonNullProps::True
  );
}

/// A spread is merged before the reader sees it, so the names it brought are
/// the names it records.
#[test]
fn reads_the_names_a_spread_brought() {
  assert_eq!(
    namespaces_of("({ root: { ...{ color: 'red' }, margin: '1px' } }).root"),
    named(&["color", "margin"])
  );
}

/// A key that is not a plain name is still a name the runtime needs, because
/// the evaluator names every key it folds.
#[test]
fn reads_a_key_that_is_not_a_plain_name() {
  assert_eq!(
    namespaces_of("({ root: { 'background-color': 'red' } }).root"),
    named(&["background-color"])
  );
}

/// The names `declared_namespaces` reads off the object `code` writes.
///
/// The reader is given the object directly, because no source reaches the
/// refusals below: the fold refuses a spread and a property that is no
/// key-value pair before it writes an object, and every producer that hands
/// back an object the fold did not rebuild writes key-value pairs only. What is
/// read here is the reader's own promise, that a property it cannot name stops
/// the build rather than going missing.
fn declared_namespaces_of(code: &str) -> Vec<Atom> {
  let props = match expr(code) {
    Expr::Object(object) => object.props,
    other => panic!("the fixture {code} is not an object literal: {other:?}"),
  };

  // Collected, because the reader is lazy and a refusal happens only where a
  // property is read.
  declared_namespaces(&props).collect()
}

/// A spread brings names this reader cannot list, so it is refused. Passed over
/// instead, every name behind it would be dropped from the keep list and the
/// null sweep would delete a namespace the runtime still reads.
#[test]
#[should_panic(expected = "The spread operator (...) is not supported in this context.")]
fn refuses_a_spread_it_cannot_name() {
  declared_namespaces_of("{ ...rest }");
}

/// A method, a getter, a setter and a shorthand name carry no name-and-value
/// pair. One case per spelling, because each is a different node and only a
/// case proves the arm takes it.
#[test]
#[should_panic(expected = "A style value can only contain an array, string or number.")]
fn refuses_a_method() {
  declared_namespaces_of("{ method() { return 1 } }");
}

#[test]
#[should_panic(expected = "A style value can only contain an array, string or number.")]
fn refuses_a_getter() {
  declared_namespaces_of("{ get color() { return 1 } }");
}

#[test]
#[should_panic(expected = "A style value can only contain an array, string or number.")]
fn refuses_a_setter() {
  declared_namespaces_of("{ set color(value) {} }");
}

/// A shorthand name holds its value in a binding beside the object, which is no
/// value this reader can read. The evaluator expands one before it writes the
/// object, so only a reader given the object directly sees it.
#[test]
#[should_panic(expected = "A style value can only contain an array, string or number.")]
fn refuses_a_shorthand_name() {
  declared_namespaces_of("{ color }");
}

/// A computed key naming no static value names no property either.
#[test]
#[should_panic(expected = "The key has no name at compile time.")]
fn refuses_a_key_with_no_name() {
  declared_namespaces_of("{ [other]: 'red' }");
}

/// The refusal comes before the names beside it are read, so a property the
/// reader cannot name stops the whole object rather than shortening it.
#[test]
#[should_panic(expected = "The spread operator (...) is not supported in this context.")]
fn refuses_before_it_names_the_properties_beside_it() {
  declared_namespaces_of("{ color: 'red', ...rest, margin: '1px' }");
}

/// Every shape a key is written in names its property, so none of them is
/// refused and none of them goes missing. Read through `as_ident` alone, the
/// last three answered nothing and the namespace behind each was dropped. A
/// quoted key is the one that arrives from a real source, on an object the fold
/// did not rebuild.
#[test]
fn names_a_property_however_its_key_is_written() {
  assert_eq!(
    declared_namespaces_of("{ color: 1, 'background-color': 2, 0: 3, ['--x']: 4 }"),
    names(&["color", "background-color", "0", "--x"])
  );
}

/// A number key names the property JavaScript names, not the digits Rust
/// prints.
#[test]
fn names_a_number_key_the_way_javascript_spells_it() {
  assert_eq!(declared_namespaces_of("{ 1e21: 'red' }"), names(&["1e+21"]));
}

/// A property declared absent declares nothing, whichever shape its key takes.
#[test]
fn leaves_out_every_property_that_declares_nothing() {
  assert_eq!(
    declared_namespaces_of("{ color: 'red', margin: null, 'padding': null }"),
    names(&["color"])
  );
}

/// Nothing to read is not something to refuse.
#[test]
fn names_nothing_of_an_object_with_no_properties() {
  assert_eq!(declared_namespaces_of("{}"), names(&[]));
}

/// A name the reader answers twice is answered twice: the reader reports what
/// the object declares and does not decide what to do about a repeat.
#[test]
fn answers_a_repeated_name_as_often_as_it_is_declared() {
  assert_eq!(
    declared_namespaces_of("{ color: 'red', color: 'blue' }"),
    names(&["color", "color"])
  );
}

/// A large object is read whole, one name per property.
#[test]
fn names_every_property_of_a_large_object() {
  let source = (0..1_000)
    .map(|index| format!("p{index}: {index}"))
    .collect::<Vec<String>>()
    .join(", ");

  assert_eq!(
    declared_namespaces_of(&format!("{{ {source} }}")).len(),
    1_000
  );
}

fn names(values: &[&str]) -> Vec<Atom> {
  values.iter().map(|value| Atom::from(*value)).collect()
}
