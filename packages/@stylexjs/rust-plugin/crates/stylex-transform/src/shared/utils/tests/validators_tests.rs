//! Tests for the refusals the compiler makes before it reads a StyleX call.

use stylex_state::{evaluate_result_value::EvaluateResultValue, state_manager::StateManager};

use super::{
  assert_valid_keyframes, assert_valid_position_try, assert_valid_properties,
  assert_valid_view_transition_class, validate_conditional_styles, validate_theme_variables,
};
use crate::shared::enums::data_structures::theme_vars::ThemeVars;
use crate::tests::support::expr;

/// The folded value `code` spells, as the evaluator would hand it over.
fn folded(code: &str) -> EvaluateResultValue {
  EvaluateResultValue::Expr(expr(code))
}

#[test]
fn a_keyframe_is_an_object_of_objects() {
  assert_valid_keyframes(
    &folded("{ from: { opacity: 0 }, to: { opacity: 1 } }"),
    &mut StateManager::default(),
  );
  assert_valid_keyframes(&folded("{}"), &mut StateManager::default());
}

/// Each step of a keyframe holds the declarations of that step, so a step that
/// is not an object declares nothing.
#[test]
#[should_panic(expected = "Every frame within a keyframes() call must be an object")]
fn refuses_a_keyframe_step_that_is_not_an_object() {
  assert_valid_keyframes(&folded("{ from: 'red' }"), &mut StateManager::default());
}

#[test]
#[should_panic(expected = "keyframes() can only accept an object")]
fn refuses_a_keyframes_argument_that_is_not_an_object() {
  assert_valid_keyframes(&folded("'red'"), &mut StateManager::default());
}

/// A value the compiler could not fold is not a keyframe it can read.
#[test]
#[should_panic(expected = "Only static values are allowed inside of a keyframes() call.")]
fn refuses_a_keyframes_argument_that_did_not_fold() {
  assert_valid_keyframes(&EvaluateResultValue::Null, &mut StateManager::default());
}

#[test]
fn a_position_try_argument_is_an_object() {
  assert_valid_position_try(&folded("{ top: 0 }"), &mut StateManager::default());
  assert_valid_view_transition_class(&folded("{}"), &mut StateManager::default());
}

#[test]
#[should_panic(expected = "positionTry() can only accept an object")]
fn refuses_a_position_try_argument_that_is_not_an_object() {
  assert_valid_position_try(&folded("'red'"), &mut StateManager::default());
}

#[test]
#[should_panic(expected = "Only static values are allowed inside of a positionTry() call.")]
fn refuses_a_position_try_argument_that_did_not_fold() {
  assert_valid_position_try(&EvaluateResultValue::Null, &mut StateManager::default());
}

#[test]
#[should_panic(expected = "viewTransitionClass() can only accept an object")]
fn refuses_a_view_transition_class_argument_that_is_not_an_object() {
  assert_valid_view_transition_class(&folded("1"), &mut StateManager::default());
}

#[test]
#[should_panic(expected = "Only static values are allowed inside of a viewTransitionClass() call.")]
fn refuses_a_view_transition_class_argument_that_did_not_fold() {
  assert_valid_view_transition_class(&EvaluateResultValue::Null, &mut StateManager::default());
}

#[test]
fn accepts_only_the_keys_a_call_names() {
  assert_valid_properties(
    &folded("{ top: 0 }"),
    &["top", "left"],
    "only top or left",
    &mut StateManager::default(),
  );
}

#[test]
#[should_panic(expected = "only top or left")]
fn refuses_a_key_a_call_does_not_name() {
  assert_valid_properties(
    &folded("{ bottom: 0 }"),
    &["top", "left"],
    "only top or left",
    &mut StateManager::default(),
  );
}

/// A value that is not an object names no keys to check, so there is nothing to
/// refuse here and the reader beside this one reports it.
#[test]
fn checks_no_key_of_a_value_that_names_none() {
  assert_valid_properties(
    &folded("'red'"),
    &["top"],
    "only top",
    &mut StateManager::default(),
  );
  assert_valid_properties(
    &EvaluateResultValue::Null,
    &["top"],
    "only top",
    &mut StateManager::default(),
  );
}

/// The first property of the object `code` spells.
fn first_pair(code: &str) -> swc_core::ecma::ast::KeyValueProp {
  match crate::tests::support::object(code).props.into_iter().next() {
    Some(prop) => match prop.prop().and_then(|prop| prop.key_value()) {
      Some(key_value) => key_value,
      None => panic!("the fixture {code} holds no key-value pair"),
    },
    None => panic!("the fixture {code} holds no property"),
  }
}

/// A condition holds declarations, a nested condition, or a name the compiler
/// resolves later.
#[test]
fn a_condition_holds_a_value_or_a_nested_condition() {
  for code in [
    "{ ':hover': { default: 'red' } }",
    "{ default: 'red' }",
    "{ '@media print': { default: 'red' } }",
    "{ 'var(--x)': 'red' }",
    "{ default: name }",
    "{ default: ['red', 'blue'] }",
  ] {
    validate_conditional_styles(&first_pair(code), &[], &mut StateManager::default());
  }
}

/// A key that names neither a pseudo selector nor an at-rule names no
/// condition, so the declarations under it would never apply.
#[test]
#[should_panic(expected = "Invalid pseudo or at-rule")]
fn refuses_a_key_that_names_no_condition() {
  validate_conditional_styles(
    &first_pair("{ hover: 'red' }"),
    &[],
    &mut StateManager::default(),
  );
}

/// One condition written twice on one path is a declaration that shadows
/// itself, which is a mistake rather than a rule.
#[test]
#[should_panic(expected = "The same pseudo selector or at-rule cannot be used more than once.")]
fn refuses_a_condition_that_is_already_in_the_path() {
  validate_conditional_styles(
    &first_pair("{ ':hover': 'red' }"),
    &[":hover".to_owned()],
    &mut StateManager::default(),
  );
}

/// A condition holds a declaration, not a shape with no CSS value.
#[test]
#[should_panic(expected = "A style value can only contain an array, string or number.")]
fn refuses_a_condition_holding_a_value_that_is_not_a_style_value() {
  validate_conditional_styles(
    &first_pair("{ ':hover': () => 1 }"),
    &[],
    &mut StateManager::default(),
  );
}

/// A theme overrides a variable group, which is named by the hash the group
/// carries. A value that is not a group cannot be overridden.
#[test]
#[should_panic(expected = "Can only override variables theme created with defineVars().")]
fn refuses_a_theme_target_that_is_not_a_variable_group() {
  validate_theme_variables(&folded("1"), &StateManager::default());
}

#[test]
#[should_panic(expected = "Can only override variables theme created with defineVars().")]
fn refuses_a_theme_target_that_carries_no_group_hash() {
  validate_theme_variables(&folded("{ color: 'red' }"), &StateManager::default());
}

/// A group hash that spells no text names no group.
#[test]
#[should_panic(expected = "Can only override variables theme created with defineVars().")]
fn refuses_a_group_hash_that_is_empty() {
  validate_theme_variables(
    &folded("{ __varGroupHash__: '' }"),
    &StateManager::default(),
  );
}

/// A group hash the compiler could not fold names no group either.
#[test]
#[should_panic(expected = "Can only override variables theme created with defineVars().")]
fn refuses_a_group_hash_that_did_not_fold() {
  validate_theme_variables(
    &folded("{ __varGroupHash__: name }"),
    &StateManager::default(),
  );
}

#[test]
fn reads_the_group_hash_a_theme_target_carries() {
  let (group_name, theme_vars) = validate_theme_variables(
    &folded("{ __varGroupHash__: 'x568ih9', color: 'var(--xcolour)' }"),
    &StateManager::default(),
  );

  assert_eq!(group_name, "x568ih9");

  match theme_vars {
    ThemeVars::Object(key_values) => assert_eq!(key_values.len(), 2),
    ThemeVars::Group(_) => panic!("an object names its own variables"),
  }
}
