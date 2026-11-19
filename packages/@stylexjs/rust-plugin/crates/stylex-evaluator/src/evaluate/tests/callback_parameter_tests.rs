//! Tests callback parameter binding and unsupported parameter or body shapes.

use super::source_evaluation::*;
use stylex_constants::constants::evaluation_errors::unsupported_expression;

const MAPPED: &str = "parts";

/// Checks the arrow's exact refusal; the enclosing call may report a different node.
#[track_caller]
fn assert_arrow_refuses(arrow: &str) {
  assert_deopt_reason_contains(arrow, &unsupported_expression("ArrowFunctionExpression"));
}

#[track_caller]
fn assert_mapping_refuses(init: &str, source: &str) {
  let described = format!("{source}` against `const {MAPPED} = {init}");

  assert_refused(
    &evaluated_in_a_module_binding(MAPPED, init, source),
    &described,
  );
}

#[test]
fn an_array_pattern_is_not_a_parameter_a_fold_binds() {
  assert_arrow_refuses("([first, second]) => first + second");
  assert_arrow_refuses("([first, [inner]]) => first + inner");

  assert_mapping_refuses(
    "[['a', 'b'], ['c', 'd']]",
    "parts.map(([first, second]) => first + second).join('-')",
  );
  assert_mapping_refuses(
    "[[1, [2]]]",
    "parts.map(([first, [inner]]) => first + inner).join('-')",
  );
}

#[test]
fn an_object_pattern_is_not_a_parameter_a_fold_binds_in_any_spelling() {
  assert_arrow_refuses("({ a }) => a");
  assert_arrow_refuses("({ a: renamed }) => renamed");
  assert_arrow_refuses("({ ['a']: computed }) => computed");

  assert_mapping_refuses(
    "[{ a: 'x' }, { a: 'y' }]",
    "parts.map(({ a }) => a).join('-')",
  );
  assert_mapping_refuses(
    "[{ a: 'x' }]",
    "parts.map(({ a: renamed }) => renamed).join('-')",
  );
  assert_mapping_refuses(
    "[{ a: 'x' }]",
    "parts.map(({ ['a']: computed }) => computed).join('-')",
  );
}

#[test]
fn a_default_makes_a_parameter_one_no_fold_binds() {
  assert_arrow_refuses("({ a = 'x' }) => a");
  assert_arrow_refuses("(each = 'z') => each");

  assert_mapping_refuses("[{}]", "parts.map(({ a = 'x' }) => a).join('-')");
  assert_mapping_refuses(
    "['a', undefined]",
    "parts.map((each = 'z') => each).join('-')",
  );
}

#[test]
fn a_rest_element_is_not_a_parameter_a_fold_binds() {
  assert_arrow_refuses("(...rest) => rest[0]");
  assert_arrow_refuses("({ ...rest }) => rest.a");

  assert_mapping_refuses("['a']", "parts.map((...rest) => rest[0]).join('-')");
  assert_mapping_refuses(
    "[{ a: 'x' }]",
    "parts.map(({ ...rest }) => rest.a).join('-')",
  );
}

#[test]
fn a_plain_name_binds_and_a_computed_read_inside_the_callback_folds() {
  assert_eq!(
    folded_in_a_module_binding(
      MAPPED,
      "['a', 'b']",
      "parts.map((key) => ({ a: 1, b: 2 })[key]).join('-')"
    ),
    "1-2"
  );
}

#[test]
fn each_half_of_the_rule_refuses_on_its_own() {
  assert_arrow_refuses("(each) => { return each; }");
  assert_arrow_refuses("([each]) => each");
}
