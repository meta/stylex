//! Small readers the transform asks about an expression.

use stylex_enums::top_level_expression::TopLevelExpressionKind;
use stylex_state::state_manager::StateManager;
use stylex_structures::top_level_expression::TopLevelExpression;
use swc_core::ecma::ast::{ModuleDecl, ModuleItem};

use crate::shared::utils::ast::helpers::{
  expr_contains_arrow, get_property_by_key, named_export_name,
};
use crate::tests::support::{expr, module};

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

#[test]
fn a_star_export_names_no_variable() {
  let state = state_exporting("export * as everything from './other';");

  assert_eq!(named_export_name(&statement_named("styles"), &state), None);
}

#[test]
fn reads_a_property_that_is_keyed_by_a_string() {
  let object = expr("{ 'background-color': 'red' }");

  assert!(get_property_by_key(&object, "background-color").is_some());
}

/// Even when the computed key would fold to the requested key.
#[test]
fn reads_no_property_under_a_computed_key() {
  assert!(get_property_by_key(&expr("{ ['color']: 'red' }"), "color").is_none());
}

/// Spreads and shorthand methods are not key-value pairs.
#[test]
fn reads_past_a_property_that_is_not_a_key_value_pair() {
  let object = expr("{ ...rest, get color() { return 1 }, size: 1 }");

  assert!(get_property_by_key(&object, "color").is_none());
  assert!(get_property_by_key(&object, "size").is_some());
}

#[test]
fn reads_no_property_of_something_that_is_not_an_object() {
  assert!(get_property_by_key(&expr("[1, 2]"), "color").is_none());
  assert!(get_property_by_key(&expr("'red'"), "color").is_none());
}

#[test]
fn stops_at_the_first_arrow() {
  assert!(expr_contains_arrow(&expr(
    "{ a: () => 1, b: c + d, e: [f, g] }"
  )));
}
