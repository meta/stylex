//! Tests for the flattening of one authored style object into the rules it
//! stands for.

use indexmap::IndexMap;
use stylex_enums::style_resolution::StyleResolution;
use stylex_evaluator::state::EvaluationState;
use stylex_state::{functions::FunctionMap, state_manager::StateManager};
use stylex_structures::stylex_options::StyleXOptions;
use swc_core::ecma::ast::{KeyValueProp, ModuleItem, Stmt};

use super::{PreRules, flatten_raw_style_object_logic};
use crate::tests::support::{module, object};

/// The properties the object `code` spells, as the flattener is given them.
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

/// A state that has read the declarations `code` spells.
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

/// A key spelled as a variable reference names the variable, which is how a
/// constant's placeholder reaches the declaration it belongs to.
#[test]
fn reads_a_variable_reference_key_as_the_variable_it_names() {
  assert_eq!(keys_of(&flatten("{ 'var(--x)': 'red' }")), ["--x"]);
}

/// The conditions written under a variable reference are filed under the
/// variable as well, so one class name carries the whole set.
#[test]
fn reads_a_variable_reference_key_carrying_conditions() {
  assert_eq!(
    keys_of(&flatten(
      "{ 'var(--x1a2b3)': { default: 'red', ':hover': 'blue' } }"
    )),
    ["--x1a2b3"]
  );
}

/// A name holding a dash is outside the class the expression spells, so the
/// key is no reference to it. The shorthand expansion one layer down asks a
/// wider question and names the property there.
#[test]
fn names_a_key_outside_the_reference_class_one_layer_down() {
  assert_eq!(
    keys_of(&flatten("{ 'var(--my-colour)': 'red' }")),
    ["--my-colour"]
  );
}

/// A bracket beside a non-space character is no variable reference, and a key
/// holding one keeps every character the author wrote.
#[test]
fn keeps_a_key_that_is_no_variable_reference_whole() {
  assert_eq!(keys_of(&flatten("{ 'a)b': 'red' }")), ["a)b"]);
}

/// The same, for a key shorter than the slice a reference is named by: reading
/// it as one cut past its end and stopped the build.
#[test]
fn keeps_a_key_shorter_than_a_reference_whole() {
  assert_eq!(keys_of(&flatten("{ ')a': 'red' }")), [")a"]);
}

/// A key can carry text outside ASCII, which a slice counted in bytes cuts
/// through.
#[test]
fn keeps_a_key_carrying_text_outside_ascii_whole() {
  assert_eq!(keys_of(&flatten("{ 'a)bé': 'red' }")), ["a)bé"]);
  assert_eq!(
    keys_of(&flatten("{ 'é var(--x1a2b3) é': 'red' }")),
    ["r(--x1a2b3) "]
  );
}

/// A character JavaScript spells with two units counts as two, so the name of
/// a key carrying one is cut where the reference implementation cuts it.
#[test]
fn counts_a_two_unit_character_as_two() {
  assert_eq!(
    keys_of(&flatten("{ '\u{1F388}var(--x1a2b3)x': 'red' }")),
    ["r(--x1a2b3)"]
  );
}

/// A pseudo selector holds brackets, and reading one as a reference named the
/// rule by text cut out of its middle.
#[test]
fn keeps_a_pseudo_selector_key_whole() {
  assert_eq!(
    keys_of(&flatten("{ ':not(.a)::before': { color: 'red' } }")),
    [":not(.a)::before_color"]
  );
}

/// An at-rule holding a bracket beside a comma is one more key that is no
/// reference.
#[test]
fn keeps_an_at_rule_key_whole() {
  assert_eq!(
    keys_of(&flatten(
      "{ '@media (min-width:1px),(max-width:2px)': { color: 'red' } }"
    )),
    ["@media (min-width:1px),(max-width:2px)_color"]
  );
}

/// A fallback list becomes one rule holding every value that survives.
#[test]
fn flattens_a_fallback_list_into_one_rule() {
  assert_eq!(keys_of(&flatten("{ color: ['red', 'blue'] }")), ["color"]);
}

/// A repeat in a fallback list says nothing the first one did not, so it goes.
#[test]
fn drops_a_repeat_from_a_fallback_list() {
  let rules = flatten("{ color: ['red', 'red'] }");

  assert_eq!(format!("{:?}", rules["color"]).matches("red").count(), 1);
}

/// A list holding only absent values declares nothing, and the property
/// survives carrying that absence so a later declaration of it is unset rather
/// than shadowed.
#[test]
fn flattens_a_list_that_empties_into_an_absence() {
  let rules = flatten("{ color: [null, null] }");

  assert!(matches!(rules["color"], PreRules::NullPreRule(_)));
}

/// Only a literal can stand in a fallback list: every entry becomes a
/// declaration of its own, and there is nothing to declare for anything else.
#[test]
#[should_panic(expected = "A style array value can only contain strings or numbers.")]
fn refuses_a_fallback_entry_that_is_not_a_literal() {
  flatten("{ color: ['red', name] }");
}

/// A template literal is read as the text it spells.
#[test]
fn reads_a_template_literal_as_its_text() {
  let rules = flatten("{ width: `10px` }");

  assert_eq!(keys_of(&rules), ["width"]);
  assert!(format!("{:?}", rules["width"]).contains("10px"));
}

/// A name is read as the value it was declared with.
#[test]
fn reads_a_name_as_the_value_it_was_declared_with() {
  let mut state = state_declaring("const red = 'red';");
  let rules = flatten_in("{ color: red }", &mut state);

  assert_eq!(keys_of(&rules), ["color"]);
  assert!(format!("{:?}", rules["color"]).contains("red"));
}

/// A name the compiler cannot resolve names no value, so there is nothing to
/// declare and the call is refused.
#[test]
#[should_panic(expected = "Only static values are allowed inside of a stylex")]
fn refuses_a_name_it_cannot_resolve() {
  flatten("{ color: red }");
}

/// `undefined` is a value rather than a name that failed to resolve, and a
/// style value position refuses it for not being a style value.
#[test]
#[should_panic(expected = "A style value can only contain an array, string or number.")]
fn refuses_an_undefined_value() {
  flatten("{ color: undefined }");
}

/// An arithmetic expression is read as the number it comes to.
#[test]
fn reads_an_arithmetic_expression_as_its_number() {
  let rules = flatten("{ width: 5 + 5 }");

  assert_eq!(keys_of(&rules), ["width"]);
  assert!(format!("{:?}", rules["width"]).contains("10"));
}

/// A call names no static value.
#[test]
#[should_panic(expected = "Only static values are allowed inside of a stylex")]
fn refuses_a_call_as_a_value() {
  flatten("{ color: makeColour() }");
}

/// A condition holds the declarations that apply under it, keyed by the
/// property they declare.
#[test]
fn flattens_a_condition_into_the_property_it_declares() {
  assert_eq!(
    keys_of(&flatten("{ color: { default: 'red', ':hover': 'blue' } }")),
    ["color"]
  );
}

/// An empty object declares nothing at all, and nothing after it does either.
#[test]
fn flattens_an_empty_object_into_nothing() {
  assert!(flatten("{ color: {} }").is_empty());
}

/// A spread inside a condition names no property, and a property that is not a
/// key-value pair declares no value. Neither is a condition the compiler can
/// read.
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

/// A hole in a fallback list carries no value at all -- not even an absent one
/// -- so the list declares what the same list written without it declares.
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

/// An absent value written on its own declares the property unset.
#[test]
fn flattens_an_absent_value_into_an_absence() {
  let rules = flatten("{ color: null }");

  assert_eq!(keys_of(&rules), ["color"]);
  assert!(matches!(rules["color"], PreRules::NullPreRule(_)));
}

/// A shorthand written absent unsets every property it stands for, not only
/// the one the author spelled.
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

/// A member read names no static value the compiler can declare, and it is
/// none of the shapes a style value is written in.
#[test]
#[should_panic(expected = "A style value can only contain an array, string or number.")]
fn refuses_a_member_read_as_a_value() {
  flatten("{ color: theme.red }");
}

/// A condition names no property of its own, so a value written straight under
/// one declares nothing. The condition holds declarations, and this object
/// holds none.
#[test]
fn declares_nothing_for_a_value_written_under_a_condition() {
  assert!(flatten("{ ':hover': 'red' }").is_empty());
}
