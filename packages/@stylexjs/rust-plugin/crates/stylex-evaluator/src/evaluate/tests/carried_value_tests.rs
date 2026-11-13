//! Tests resolved values carried into calls, including key order, UTF-16 and size limits.

use super::source_evaluation::*;
use stylex_constants::constants::evaluation_errors::{
  bound_value_has_too_many_entries, bound_value_too_large,
};
use stylex_structures::stylex_options::StyleXOptions;

const CARRIED: &str = "carried";

#[track_caller]
fn folded_carrying(init: &str, source: &str) -> String {
  folded_in_a_module_binding(CARRIED, init, source)
}

#[test]
fn every_primitive_crosses_as_the_value_it_stands_for() {
  assert_eq!(folded_carrying("['a', 'b']", "carried.join('-')"), "a-b");
  assert_eq!(folded_carrying("[1, 2]", "carried.join('-')"), "1-2");
  assert_eq!(
    folded_carrying("[true, false]", "carried.join('-')"),
    "true-false"
  );
  assert_eq!(folded_carrying("'abc'", "carried.toUpperCase()"), "ABC");
}

#[test]
fn the_two_absent_values_cross_as_themselves() {
  assert_eq!(
    folded_carrying("[null, undefined]", "carried.join('-')"),
    "-"
  );
  assert_eq!(
    folded_carrying("[null, 'a', undefined]", "carried.join(',')"),
    ",a,"
  );
}

#[test]
fn a_nested_array_crosses_with_its_nesting_intact() {
  assert_eq!(
    folded_carrying("[['a'], ['b', 'c']]", "carried.flat().join('-')"),
    "a-b-c"
  );
  assert_eq!(
    folded_carrying("[[1, [2]]]", "carried.flat(2).join('-')"),
    "1-2"
  );
}

#[test]
fn an_object_crosses_with_its_own_keys_in_order() {
  assert_eq!(
    folded_carrying("({ a: 1, b: 2 })", "Object.keys(carried).join('-')"),
    "a-b"
  );
  assert_eq!(
    folded_carrying("({ b: 1, a: 2 })", "Object.keys(carried).join('-')"),
    "b-a"
  );
  assert_eq!(
    folded_carrying("({ a: { b: 'x' } })", "Object.keys(carried.a).join('-')"),
    "b"
  );
  assert_eq!(
    folded_carrying("({})", "Object.keys(carried).length + ''"),
    "0"
  );
}

#[test]
fn a_prototype_key_is_not_carried_as_a_property() {
  assert_eq!(
    folded_carrying(
      "({ __proto__: null, a: 1 })",
      "Object.keys(carried).join('-')"
    ),
    "a"
  );
}

#[test]
fn a_value_past_the_character_ceiling_refuses_by_name() {
  let mut options = StyleXOptions::default();

  options.core.max_folded_characters = 4;

  let result =
    evaluated_in_a_module_binding_under(options, CARRIED, "'abcdefgh'", "carried.toUpperCase()");

  assert_refused_with(
    &result,
    "a value past the character ceiling",
    &bound_value_too_large(CARRIED, 4),
  );
}

#[test]
fn a_value_past_the_entry_ceiling_refuses_by_name() {
  let mut options = StyleXOptions::default();

  options.core.max_folded_entries = 2;

  for init in ["[1, 2, 3]", "({ a: 1, b: 2, c: 3 })"] {
    let result = evaluated_in_a_module_binding_under(
      options.clone(),
      CARRIED,
      init,
      "Object.keys(carried).join('-')",
    );

    assert_refused_with(&result, init, &bound_value_has_too_many_entries(CARRIED, 2));
  }
}

#[test]
fn a_value_inside_the_ceilings_still_crosses() {
  let mut options = StyleXOptions::default();

  options.core.max_folded_characters = 8;
  options.core.max_folded_entries = 2;

  assert_eq!(
    folded_text_of(
      evaluated_in_a_module_binding_under(options, CARRIED, "['a', 'b']", "carried.join('-')"),
      "a value inside the ceilings",
    ),
    "a-b"
  );
}

#[test]
fn a_key_crosses_as_the_name_the_language_reads_it_as() {
  assert_eq!(folded_carrying("{ ab: 1 }", "carried.ab + ''"), "1");
  assert_eq!(folded_carrying("{ 'a-b': 1 }", "carried['a-b'] + ''"), "1");
  assert_eq!(folded_carrying("{ 2: 3 }", "carried[2] + ''"), "3");
}

#[test]
fn a_big_integer_key_is_carried_as_its_string() {
  let source = "[{ 1n: 1 }].map((entry) => entry)";

  assert_folds_to_a_value(source);
  assert_folds_to_number("[{ 1n: 1 }].map((entry) => entry)[0][1]", 1.0);
  assert_folds_to_number("[{ 1n: 1 }].map((entry) => entry)[0]['1']", 1.0);

  assert_folds_to_number("[{ 1: 1 }].map((entry) => entry)[0][1]", 1.0);
}

#[test]
fn a_value_with_no_javascript_form_is_handed_back() {
  let fns = a_function_fold();

  for source in [
    format!("[{FOLD_FUNCTION}].map((entry) => entry)"),
    format!("[{FOLD_NAMESPACE}].map((entry) => entry)"),
  ] {
    assert_refused(&evaluated_against(&fns, &source), &source);
  }

  let source = "['a'].map((entry) => entry).join('')";

  assert_eq!(folded_text_of(evaluated_against(&fns, source), source), "a");
}

#[test]
fn a_name_read_twice_crosses_once() {
  assert_eq!(
    folded_carrying("['a', 'b']", "carried.concat(carried).join('-')"),
    "a-b-a-b"
  );
}

#[test]
fn an_unpaired_surrogate_crosses_as_the_code_unit_it_is() {
  assert_eq!(
    folded_carrying(
      "'\\u{D800}b'",
      "carried.charCodeAt(0).toString(16) + carried.length"
    ),
    "d8002"
  );
}

#[test]
fn a_bare_primitive_crosses_on_its_own() {
  for (init, source, expected) in [
    ("2", "'ab'.repeat(carried)", "abab"),
    ("true", "String(carried)", "true"),
    ("false", "carried.toString()", "false"),
    ("null", "String(carried)", "null"),
    ("undefined", "String(carried)", "undefined"),
  ] {
    assert_eq!(
      folded_carrying(init, source),
      expected,
      "wrong answer for `{}` carrying `{}`",
      source,
      init
    );
  }
}
