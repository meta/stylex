//! Flattening one authored style object into the rules it stands for.

use indexmap::IndexMap;
use stylex_enums::style_resolution::StyleResolution;
use stylex_evaluator::state::EvaluationState;
use stylex_state::{functions::FunctionMap, state_manager::StateManager};
use stylex_structures::stylex_options::StyleXOptions;
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
fn flattens_a_plain_declaration() {
  assert_eq!(keys_of(&flatten("{ color: 'red' }")), ["color"]);
}

/// This is how a constant's placeholder reaches its declaration.
#[test]
fn reads_a_variable_reference_key_as_the_variable_it_names() {
  assert_eq!(keys_of(&flatten("{ 'var(--x)': 'red' }")), ["--x"]);
}

/// So one class name carries the whole set.
#[test]
fn reads_a_variable_reference_key_carrying_conditions() {
  assert_eq!(
    keys_of(&flatten(
      "{ 'var(--x1a2b3)': { default: 'red', ':hover': 'blue' } }"
    )),
    ["--x1a2b3"]
  );
}

/// A dashed name is outside the reference class; the shorthand expansion names it one layer down.
#[test]
fn names_a_key_outside_the_reference_class_one_layer_down() {
  assert_eq!(
    keys_of(&flatten("{ 'var(--my-colour)': 'red' }")),
    ["--my-colour"]
  );
}

#[test]
fn keeps_a_key_that_is_no_variable_reference_whole() {
  assert_eq!(keys_of(&flatten("{ 'a)b': 'red' }")), ["a)b"]);
}

/// Regression: slicing it as a reference cut past its end.
#[test]
fn keeps_a_key_shorter_than_a_reference_whole() {
  assert_eq!(keys_of(&flatten("{ ')a': 'red' }")), [")a"]);
}

/// A byte-counted slice would cut through it.
#[test]
fn keeps_a_key_carrying_text_outside_ascii_whole() {
  assert_eq!(keys_of(&flatten("{ 'a)bé': 'red' }")), ["a)bé"]);
  assert_eq!(
    keys_of(&flatten("{ 'é var(--x1a2b3) é': 'red' }")),
    ["r(--x1a2b3) "]
  );
}

/// Cut where the reference implementation cuts (UTF-16 units).
#[test]
fn counts_a_two_unit_character_as_two() {
  assert_eq!(
    keys_of(&flatten("{ '\u{1F388}var(--x1a2b3)x': 'red' }")),
    ["r(--x1a2b3)"]
  );
}

#[test]
fn keeps_a_pseudo_selector_key_whole() {
  assert_eq!(
    keys_of(&flatten("{ ':not(.a)::before': { color: 'red' } }")),
    [":not(.a)::before_color"]
  );
}

#[test]
fn keeps_an_at_rule_key_whole() {
  assert_eq!(
    keys_of(&flatten(
      "{ '@media (min-width:1px),(max-width:2px)': { color: 'red' } }"
    )),
    ["@media (min-width:1px),(max-width:2px)_color"]
  );
}

#[test]
fn flattens_a_fallback_list_into_one_rule() {
  assert_eq!(keys_of(&flatten("{ color: ['red', 'blue'] }")), ["color"]);
}

#[test]
fn drops_a_repeat_from_a_fallback_list() {
  let rules = flatten("{ color: ['red', 'red'] }");

  assert_eq!(format!("{:?}", rules["color"]).matches("red").count(), 1);
}

/// The property survives carrying the absence, so a later declaration is unset, not shadowed.
#[test]
fn flattens_a_list_that_empties_into_an_absence() {
  let rules = flatten("{ color: [null, null] }");

  assert!(matches!(rules["color"], PreRules::NullPreRule(_)));
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

/// `undefined` is a value, not an unresolved name, and is refused as a non-style value.
#[test]
#[should_panic(expected = "A style value can only contain an array, string or number.")]
fn refuses_an_undefined_value() {
  flatten("{ color: undefined }");
}

#[test]
fn reads_an_arithmetic_expression_as_its_number() {
  let rules = flatten("{ width: 5 + 5 }");

  assert_eq!(keys_of(&rules), ["width"]);
  assert!(format!("{:?}", rules["width"]).contains("10"));
}

#[test]
#[should_panic(expected = "Only static values are allowed inside of a stylex")]
fn refuses_a_call_as_a_value() {
  flatten("{ color: makeColour() }");
}

#[test]
fn flattens_a_condition_into_the_property_it_declares() {
  assert_eq!(
    keys_of(&flatten("{ color: { default: 'red', ':hover': 'blue' } }")),
    ["color"]
  );
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
fn flattens_an_absent_value_into_an_absence() {
  let rules = flatten("{ color: null }");

  assert_eq!(keys_of(&rules), ["color"]);
  assert!(matches!(rules["color"], PreRules::NullPreRule(_)));
}

#[test]
fn flattens_an_absent_shorthand_into_an_absence_for_each_property() {
  let mut state = StateManager::new(
    StyleXOptions::default().with_style_resolution(StyleResolution::ApplicationOrder),
  );
  let rules = flatten_in("{ margin: null }", &mut state);

  assert!(rules.len() > 1, "the shorthand expanded to {rules:?}");
  assert!(
    rules
      .values()
      .all(|rule| matches!(rule, PreRules::NullPreRule(_))),
    "the expansion declares a value: {rules:?}"
  );
}

#[test]
#[should_panic(expected = "A style value can only contain an array, string or number.")]
fn refuses_a_member_read_as_a_value() {
  flatten("{ color: theme.red }");
}

/// A condition holds declarations and names no property of its own.
#[test]
fn declares_nothing_for_a_value_written_under_a_condition() {
  assert!(flatten("{ ':hover': 'red' }").is_empty());
}
