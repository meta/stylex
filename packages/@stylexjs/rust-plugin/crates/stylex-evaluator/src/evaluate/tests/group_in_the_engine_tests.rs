//! Tests variable groups carried through callbacks, conversions and result containers.

use std::rc::Rc;

use super::source_evaluation::*;
use stylex_state::{
  evaluate_result_value::EvaluateResultValue,
  functions::{FunctionMap, FunctionType},
  theme_ref::ThemeRef,
};

const GROUP: &str = "colors";

fn map_holding_a_group() -> FunctionMap {
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

#[track_caller]
fn folded_reading_the_group(source: &str) -> String {
  folded_text_of(evaluated_against(&map_holding_a_group(), source), source)
}

#[test]
fn a_token_read_inside_a_callback_names_the_same_variable() {
  assert_eq!(
    folded_reading_the_group("['a'].map(() => colors.primary).join('-')"),
    "var(--x1ineb92)"
  );
  assert_eq!(
    folded_reading_the_group("[colors.primary].join('')"),
    "var(--x1ineb92)"
  );
}

#[test]
fn a_token_read_by_a_bound_key_resolves_per_element() {
  assert_eq!(
    folded_reading_the_group("['a', 'b'].map((key) => colors[key]).join('-')"),
    "var(--xvepf95)-var(--x1as6utw)"
  );
}

#[test]
fn a_nested_token_is_read_by_its_whole_path_inside_a_callback() {
  assert_eq!(
    folded_reading_the_group("['a'].map(() => colors.brand.primary).join('-')"),
    "var(--x1tr9ywo)"
  );
}

#[test]
fn a_group_coerced_inside_a_callback_answers_its_own_hash() {
  assert_eq!(
    folded_reading_the_group("['a'].map(() => String(colors)).join('')"),
    "xop34xu"
  );
}

#[test]
fn a_group_handed_back_by_a_conversion_is_still_the_group() {
  let fns = map_holding_a_group();

  assert!(
    matches!(
      folded_value_of(evaluated_against(&fns, "Object(colors)"), "Object(colors)"),
      EvaluateResultValue::ThemeRef(_)
    ),
    "expected the group itself back"
  );

  assert_eq!(
    folded_text_of(
      evaluated_against(&fns, "Object(colors).primary"),
      "Object(colors).primary"
    ),
    "var(--x1ineb92)"
  );
}

#[test]
fn a_token_read_off_a_group_handed_to_a_callback_names_the_same_variable() {
  assert_eq!(
    folded_reading_the_group(&format!("[{GROUP}].map((group) => group.primary).join('')")),
    "var(--x1ineb92)"
  );
}

/// Group joins deliberately refuse, matching rust-plugin PR #2008 rather than Babel's group hash.
#[test]
fn a_group_inside_a_folded_answer_carries_its_own_text() {
  let source = format!("[{GROUP}].map((group) => group).join('')");

  assert_refused(&evaluated_against(&map_holding_a_group(), &source), &source);
}

#[test]
fn a_group_carried_out_inside_an_array_answers_its_own_text() {
  let source = format!("[{GROUP}].map((group) => group)[0]");

  assert!(
    matches!(
      folded_value_of(evaluated_against(&map_holding_a_group(), &source), &source),
      EvaluateResultValue::ThemeRef(_)
    ),
    "expected the group itself back from `{source}`"
  );
}

#[test]
fn a_group_carried_out_under_a_key_answers_its_own_text() {
  let source = format!("Object.fromEntries([['a', {GROUP}]]).a");

  assert!(
    matches!(
      folded_value_of(evaluated_against(&map_holding_a_group(), &source), &source),
      EvaluateResultValue::ThemeRef(_)
    ),
    "expected the group itself back from `{source}`"
  );
}
