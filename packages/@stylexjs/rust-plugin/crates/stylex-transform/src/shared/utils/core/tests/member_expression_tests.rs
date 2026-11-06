//! The reader that records which style namespaces a bailed-out `stylex.props` call still
//! needs at runtime.

use std::rc::Rc;

use stylex_enums::style_vars_to_keep::NonNullProps;
use stylex_state::{functions::FunctionMap, state_manager::StateManager};
use swc_core::{atoms::Atom, ecma::ast::Expr};

use crate::shared::utils::core::member_expression::{declared_namespaces, member_expression};
use crate::tests::support::expr;

/// Reads `code` as one member expression, starting at the first argument with no bail-out.
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

#[test]
fn names_every_property_the_namespace_declares() {
  assert_eq!(
    namespaces_of("({ root: { color: 'red', margin: '1px' } }).root"),
    named(&["color", "margin"])
  );
}

#[test]
fn leaves_out_a_property_that_declares_nothing() {
  assert_eq!(
    namespaces_of("({ root: { color: 'red', margin: null } }).root"),
    named(&["color"])
  );
}

/// Regression: read through `as_ident` it answered nothing and the null sweep dropped a name
/// the runtime reads. The reference lists the folded object's keys, quoted or bare.
#[test]
fn names_a_property_whose_key_has_to_be_quoted() {
  assert_eq!(
    namespaces_of("({ root: { '--my-color': 'red', color: 'blue' } }).root"),
    named(&["--my-color", "color"])
  );
}

/// A number key names the property JavaScript spells for that number.
#[test]
fn names_a_property_whose_key_is_not_written_as_a_name() {
  assert_eq!(
    namespaces_of("({ root: { 0: 'red', 1e21: 'blue', ['--x']: 'green' } }).root"),
    named(&["0", "1e+21", "--x"])
  );
}

/// The reader narrows on the value, not on how the key was written.
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

#[test]
fn everything_past_the_bail_out_point_is_needed() {
  assert_eq!(
    read("({ root: { color: 'red' } }).root", named(&[]), Some(0), 1),
    NonNullProps::True
  );
}

#[test]
fn an_expression_that_does_not_fold_needs_everything() {
  assert_eq!(namespaces_of("unknown.root"), NonNullProps::True);
}

/// The evaluator merges spreads away and refuses non-name keys and valueless props.
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

/// Spreads are merged before the reader sees them.
#[test]
fn reads_the_names_a_spread_brought() {
  assert_eq!(
    namespaces_of("({ root: { ...{ color: 'red' }, margin: '1px' } }).root"),
    named(&["color", "margin"])
  );
}

/// The evaluator names every key it folds.
#[test]
fn reads_a_key_that_is_not_a_plain_name() {
  assert_eq!(
    namespaces_of("({ root: { 'background-color': 'red' } }).root"),
    named(&["background-color"])
  );
}

/// Given the object directly, since no source reaches these refusals: an unnameable property
/// must stop the build rather than go missing.
fn declared_namespaces_of(code: &str) -> Vec<Atom> {
  let props = match expr(code) {
    Expr::Object(object) => object.props,
    other => panic!("the fixture {code} is not an object literal: {other:?}"),
  };

  // Collected: the reader is lazy and refuses only where a property is read.
  declared_namespaces(&props).collect()
}

/// Skipping it would drop names from the keep list and the null sweep would delete them.
#[test]
#[should_panic(expected = "The spread operator (...) is not supported in this context.")]
fn refuses_a_spread_it_cannot_name() {
  declared_namespaces_of("{ ...rest }");
}

/// One case per spelling, because each is a different node.
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

/// The evaluator expands shorthands first, so only a direct caller sees one.
#[test]
#[should_panic(expected = "A style value can only contain an array, string or number.")]
fn refuses_a_shorthand_name() {
  declared_namespaces_of("{ color }");
}

#[test]
#[should_panic(expected = "The key has no name at compile time.")]
fn refuses_a_key_with_no_name() {
  declared_namespaces_of("{ [other]: 'red' }");
}

/// An unnameable property stops the whole object rather than shortening it.
#[test]
#[should_panic(expected = "The spread operator (...) is not supported in this context.")]
fn refuses_before_it_names_the_properties_beside_it() {
  declared_namespaces_of("{ color: 'red', ...rest, margin: '1px' }");
}

/// Regression: through `as_ident` alone the last three were dropped. Quoted keys arrive from
/// real source on objects the fold did not rebuild.
#[test]
fn names_a_property_however_its_key_is_written() {
  assert_eq!(
    declared_namespaces_of("{ color: 1, 'background-color': 2, 0: 3, ['--x']: 4 }"),
    names(&["color", "background-color", "0", "--x"])
  );
}

#[test]
fn names_a_number_key_the_way_javascript_spells_it() {
  assert_eq!(declared_namespaces_of("{ 1e21: 'red' }"), names(&["1e+21"]));
}

#[test]
fn leaves_out_every_property_that_declares_nothing() {
  assert_eq!(
    declared_namespaces_of("{ color: 'red', margin: null, 'padding': null }"),
    names(&["color"])
  );
}

#[test]
fn names_nothing_of_an_object_with_no_properties() {
  assert_eq!(declared_namespaces_of("{}"), names(&[]));
}

/// The reader reports what is declared and leaves repeats to the caller.
#[test]
fn answers_a_repeated_name_as_often_as_it_is_declared() {
  assert_eq!(
    declared_namespaces_of("{ color: 'red', color: 'blue' }"),
    names(&["color", "color"])
  );
}

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
