//! Tests for the small readers the transform asks about an expression.

use stylex_enums::top_level_expression::TopLevelExpressionKind;
use stylex_state::state_manager::StateManager;
use stylex_structures::top_level_expression::TopLevelExpression;
use swc_core::ecma::{
  ast::{ModuleDecl, ModuleItem},
  atoms::Atom,
};

use crate::shared::utils::ast::helpers::{
  expr_contains_arrow, get_property_by_key, named_export_name,
};
use crate::tests::support::{expr, module};

/// A state that has read the export statements `code` spells.
fn state_exporting(code: &str) -> StateManager {
  let mut state = StateManager::default();

  for item in module(code).body {
    if let ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(named)) = item {
      state.named_exports.insert(named);
    }
  }

  state
}

fn statement_named(name: &str) -> TopLevelExpression {
  TopLevelExpression(TopLevelExpressionKind::Stmt, expr("1"), Some(name.into()))
}

/// An expression the compiler already read as a named export needs no export
/// statement to confirm it, and it answers the name it is written with.
#[test]
fn a_named_export_answers_its_own_name() {
  let expression = TopLevelExpression(
    TopLevelExpressionKind::NamedExport,
    expr("1"),
    Some("styles".into()),
  );

  assert_eq!(
    named_export_name(&expression, &StateManager::default()).map(Atom::as_str),
    Some("styles")
  );
}

/// Anything else is looked up by the name it is bound to. An expression bound
/// to nothing has no name to look up, which is the one shape a default export
/// takes.
#[test]
fn an_unnamed_statement_answers_no_name() {
  let expression = TopLevelExpression(TopLevelExpressionKind::Stmt, expr("1"), None);

  assert_eq!(
    named_export_name(&expression, &StateManager::default()),
    None
  );
}

#[test]
fn a_statement_answers_the_name_a_statement_exports() {
  let state = state_exporting("const styles = 1; export { styles };");

  assert_eq!(
    named_export_name(&statement_named("styles"), &state).map(Atom::as_str),
    Some("styles")
  );
}

#[test]
fn a_statement_no_export_names_answers_no_name() {
  let state = state_exporting("const styles = 1; export { other };");

  assert_eq!(named_export_name(&statement_named("styles"), &state), None);
}

/// An export under another name is a different export, and so is one that
/// takes its value from another module. Neither names this expression.
#[test]
fn a_renamed_or_re_exported_name_is_not_this_export() {
  let renamed = state_exporting("const styles = 1; export { styles as theme };");
  let re_exported = state_exporting("export { styles } from './other';");

  assert_eq!(
    named_export_name(&statement_named("styles"), &renamed),
    None
  );
  assert_eq!(
    named_export_name(&statement_named("styles"), &re_exported),
    None
  );
}

/// A star export names nothing, so it decides nothing about this expression.
#[test]
fn a_star_export_names_no_variable() {
  let state = state_exporting("export * as everything from './other';");

  assert_eq!(named_export_name(&statement_named("styles"), &state), None);
}

#[test]
fn reads_a_property_that_is_keyed_by_a_name() {
  let object = expr("{ color: 'red' }");

  assert!(get_property_by_key(&object, "color").is_some());
}

/// A key written as a string names the same property as one written as a name.
#[test]
fn reads_a_property_that_is_keyed_by_a_string() {
  let object = expr("{ 'background-color': 'red' }");

  assert!(get_property_by_key(&object, "background-color").is_some());
}

#[test]
fn reads_no_property_under_a_key_the_object_does_not_hold() {
  assert!(get_property_by_key(&expr("{ color: 'red' }"), "size").is_none());
  assert!(get_property_by_key(&expr("{}"), "color").is_none());
}

/// A computed key spells no name to compare, so it names no property here even
/// when it would fold to the key that is asked for.
#[test]
fn reads_no_property_under_a_computed_key() {
  assert!(get_property_by_key(&expr("{ ['color']: 'red' }"), "color").is_none());
}

/// A spread and a shorthand method are not key-value pairs, and neither holds
/// the value the caller asks for.
#[test]
fn reads_past_a_property_that_is_not_a_key_value_pair() {
  let object = expr("{ ...rest, get color() { return 1 }, size: 1 }");

  assert!(get_property_by_key(&object, "color").is_none());
  assert!(get_property_by_key(&object, "size").is_some());
}

/// Only an object holds properties to read.
#[test]
fn reads_no_property_of_something_that_is_not_an_object() {
  assert!(get_property_by_key(&expr("[1, 2]"), "color").is_none());
  assert!(get_property_by_key(&expr("'red'"), "color").is_none());
}

#[test]
fn finds_an_arrow_wherever_it_is_written() {
  assert!(expr_contains_arrow(&expr("() => 1")));
  assert!(expr_contains_arrow(&expr("{ color: () => 1 }")));
  assert!(expr_contains_arrow(&expr("a ? (() => 1) : 2")));
  assert!(expr_contains_arrow(&expr("[1, [2, () => 3]]")));
  assert!(expr_contains_arrow(&expr("tag`${() => 1}`")));
  assert!(expr_contains_arrow(&expr("f(1, () => 2)")));
}

/// The search stops at the first arrow, and everything written after it is left
/// unread.
#[test]
fn stops_at_the_first_arrow() {
  assert!(expr_contains_arrow(&expr(
    "{ a: () => 1, b: c + d, e: [f, g] }"
  )));
}

#[test]
fn finds_no_arrow_where_none_is_written() {
  assert!(!expr_contains_arrow(&expr("{ color: 'red' }")));
  assert!(!expr_contains_arrow(&expr("function () { return 1 }")));
  assert!(!expr_contains_arrow(&expr("a + b")));
}
