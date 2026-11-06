//! Flattening one authored style object into the rules it stands for.

use indexmap::IndexMap;
use stylex_evaluator::state::EvaluationState;
use stylex_state::{functions::FunctionMap, state_manager::StateManager};
use swc_core::ecma::ast::{KeyValueProp, ModuleItem, Stmt};

use super::{PreRules, flatten_raw_style_object_logic};
use crate::tests::support::{module, object};

fn pairs(code: &str) -> Vec<KeyValueProp> {
  object(code)
    .props
    .into_iter()
    .filter_map(|prop| prop.prop().and_then(|prop| prop.key_value()))
    .collect()
}

/// The rules the object `code` flattens to, in the order it wrote them.
fn flatten(code: &str) -> IndexMap<String, PreRules> {
  flatten_in(code, &mut StateManager::default())
}

fn flatten_in(code: &str, traversal_state: &mut StateManager) -> IndexMap<String, PreRules> {
  flatten_raw_style_object_logic(
    &pairs(code),
    &mut vec![],
    &mut EvaluationState::default(),
    traversal_state,
    &FunctionMap::default(),
  )
}

fn state_declaring(code: &str) -> StateManager {
  let mut state = StateManager::default();

  for item in module(code).body {
    if let ModuleItem::Stmt(Stmt::Decl(swc_core::ecma::ast::Decl::Var(declaration))) = item {
      for declarator in declaration.decls {
        state.push_declaration(declarator);
      }
    }
  }

  state
}

fn keys_of(rules: &IndexMap<String, PreRules>) -> Vec<String> {
  rules.keys().cloned().collect()
}

#[test]
#[should_panic(expected = "A style array value can only contain strings or numbers.")]
fn refuses_a_fallback_entry_that_is_not_a_literal() {
  flatten("{ color: ['red', name] }");
}

#[test]
fn reads_a_template_literal_as_its_text() {
  let rules = flatten("{ width: `10px` }");

  assert_eq!(keys_of(&rules), ["width"]);
  assert!(format!("{:?}", rules["width"]).contains("10px"));
}

#[test]
fn reads_a_name_as_the_value_it_was_declared_with() {
  let mut state = state_declaring("const red = 'red';");
  let rules = flatten_in("{ color: red }", &mut state);

  assert_eq!(keys_of(&rules), ["color"]);
  assert!(format!("{:?}", rules["color"]).contains("red"));
}

#[test]
#[should_panic(expected = "Only static values are allowed inside of a stylex")]
fn refuses_a_name_it_cannot_resolve() {
  flatten("{ color: red }");
}

#[test]
fn reads_an_arithmetic_expression_as_its_number() {
  let rules = flatten("{ width: 5 + 5 }");

  assert_eq!(keys_of(&rules), ["width"]);
  assert!(format!("{:?}", rules["width"]).contains("10"));
}

#[test]
fn flattens_an_empty_object_into_nothing() {
  assert!(flatten("{ color: {} }").is_empty());
}

/// A spread or a non-key-value prop inside a condition.
#[test]
#[should_panic(expected = "Only static values are allowed inside of a stylex")]
fn refuses_a_spread_inside_a_condition() {
  flatten("{ color: { ...rest } }");
}

#[test]
#[should_panic(expected = "Only static values are allowed inside of a stylex")]
fn refuses_a_method_inside_a_condition() {
  flatten("{ color: { default() { return 'red' } } }");
}

/// A hole carries no value at all, not even an absent one.
#[test]
fn skips_a_hole_in_a_fallback_list() {
  let with_hole = flatten("{ color: ['red', , 'blue'] }");
  let without = flatten("{ color: ['red', 'blue'] }");

  assert_eq!(keys_of(&with_hole), ["color"]);
  assert_eq!(
    format!("{:?}", with_hole["color"]),
    format!("{:?}", without["color"])
  );
}

#[test]
#[should_panic(expected = "A style value can only contain an array, string or number.")]
fn refuses_a_member_read_as_a_value() {
  flatten("{ color: theme.red }");
}
