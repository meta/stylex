//! Tests arrow invocation, positional binding and callback-body failures.

use std::rc::Rc;

use super::source_evaluation::*;
use stylex_constants::constants::evaluation_errors::unsupported_expression;
use stylex_state::{
  functions::{FunctionMap, FunctionType},
  theme_ref::ThemeRef,
};

#[test]
fn an_applied_arrow_answers_what_its_body_folds_to() {
  assert_eq!(
    folded_in_a_module_binding("pick", "(color) => color", "pick('red')"),
    "red"
  );
  assert_eq!(
    folded_in_a_module_binding("join", "(a, b) => a + b", "join('re', 'd')"),
    "red"
  );
  assert_eq!(
    folded_in_a_module_binding("red", "() => 'red'", "red()"),
    "red"
  );
}

#[test]
fn a_parameter_with_no_argument_is_left_unbound() {
  assert_eq!(
    folded_in_a_module_binding("first", "(a, b) => a", "first('red')"),
    "red"
  );
}

#[test]
fn an_array_argument_binds_in_the_form_a_fold_answers_it_in() {
  assert_eq!(
    folded_in_a_module_binding("dashed", "(parts) => parts.join('-')", "dashed(['a', 'b'])"),
    "a-b"
  );
  assert_eq!(
    folded_in_a_module_binding(
      "dashed",
      "(parts) => parts.join('-')",
      "dashed(['a', 'b'].map((part) => part))"
    ),
    "a-b"
  );
}

#[test]
fn a_body_that_folds_to_an_array_answers_the_array() {
  assert_eq!(
    folded_in_a_module_binding(
      "twice",
      "(color) => [color, color]",
      "twice('red').join('-')"
    ),
    "red-red"
  );
}

#[test]
fn an_argument_with_no_form_costs_a_body_that_never_reads_it_nothing() {
  assert_eq!(
    folded_in_a_module_binding("red", "(fn) => 'red'", "red(() => 1)"),
    "red"
  );
}

#[test]
fn an_argument_with_no_form_is_handed_back_by_a_body_that_reads_it() {
  folded_value_of(
    evaluated_in_a_module_binding("echo", "(fn) => fn", "echo(() => 1)"),
    "echo(() => 1)",
  );
  assert_eq!(
    folded_in_a_module_binding("echo", "(fn) => fn", "typeof echo(() => 1)"),
    "function"
  );
}

/// Babel throws unresolved callback-body references at invocation (`lib/index.js:6451`).
#[test]
#[should_panic(expected = "Referenced constant is not defined.")]
fn a_body_that_folds_to_nothing_throws_with_every_argument_bound() {
  evaluated_in_a_module_binding("shadow", "(color) => missingName", "shadow('red')");
}

#[test]
fn a_parameter_the_binding_cannot_name_leaves_the_arrow_unapplied() {
  for (init, source) in [
    ("({ color }) => color", "pick({ color: 'red' })"),
    ("([first]) => first", "pick(['red'])"),
    ("(...rest) => rest", "pick('red')"),
    ("(color = 'red') => color", "pick()"),
  ] {
    assert_refused_in_a_module_binding(
      "pick",
      init,
      source,
      &unsupported_expression("ArrowFunctionExpression"),
    );
  }
}

#[test]
fn a_block_bodied_arrow_is_not_applied() {
  assert_refused_in_a_module_binding(
    "pick",
    "(color) => { return color; }",
    "pick('red')",
    &unsupported_expression("ArrowFunctionExpression"),
  );
}

#[test]
fn a_group_argument_binds_through_its_own_factory() {
  let fns = a_function_map_holding_a_group();
  let source = format!("pick({GROUP})");

  assert_eq!(
    folded_text_of(
      evaluated_in_a_state(
        |state| state.push_declaration(a_declaration_of("pick", "(group) => group.primary")),
        &fns,
        &source,
      ),
      &source,
    ),
    "var(--x1ineb92)"
  );
}

#[test]
fn a_body_that_folds_to_an_array_answers_it_as_a_literal() {
  assert_eq!(
    folded_in_a_module_binding("twice", "(color) => [color, color]", "twice('red')[1]"),
    "red"
  );
}

const GROUP: &str = "colors";

fn a_function_map_holding_a_group() -> FunctionMap {
  let theme = ThemeRef::new("vars.stylex.js", "vars", "x");
  let mut fns = FunctionMap::default();

  fns.identifiers.insert(
    GROUP.into(),
    Box::new(folded_entry(
      FunctionType::ThemeRefMapper(Rc::new(move || theme.clone())),
      false,
    )),
  );

  fns
}
