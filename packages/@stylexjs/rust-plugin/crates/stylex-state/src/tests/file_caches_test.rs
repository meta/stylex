use std::rc::Rc;

use crate::flat_compiled_styles_value::FlatCompiledStylesValue;
use crate::functions::{FunctionMap, RuleCallHelpers};
use crate::tests::prelude::test_state as state;
use crate::types::FlatCompiledStyles;

fn compiled_styles(key: &str, value: FlatCompiledStylesValue) -> Rc<FlatCompiledStyles> {
  let mut values = FlatCompiledStyles::default();

  values.insert(key.to_string(), Rc::new(value));

  Rc::new(values)
}

#[test]
fn a_short_filename_is_worked_out_once_and_kept() {
  let mut state = state();

  assert_eq!(state.cached_short_filename("/repo/src/app.js"), None);

  state.insert_cached_short_filename("/repo/src/app.js".to_string(), "app.js".to_string());

  assert_eq!(
    state.cached_short_filename("/repo/src/app.js"),
    Some("app.js")
  );
  assert_eq!(state.cached_short_filename("/repo/src/other.js"), None);
}

#[test]
fn a_short_filename_recorded_twice_keeps_the_second() {
  let mut state = state();

  state.insert_cached_short_filename("/repo/src/app.js".to_string(), "app.js".to_string());
  state.insert_cached_short_filename("/repo/src/app.js".to_string(), "src/app.js".to_string());

  assert_eq!(
    state.cached_short_filename("/repo/src/app.js"),
    Some("src/app.js")
  );
}

#[test]
fn the_default_marker_values_are_built_once_and_kept() {
  let mut state = state();

  assert_eq!(state.cached_default_marker_values(), None);

  let values = compiled_styles("$$css", FlatCompiledStylesValue::Bool(true));

  state.insert_cached_default_marker_values(Rc::clone(&values));

  match state.cached_default_marker_values() {
    Some(read_back) => {
      assert!(Rc::ptr_eq(read_back, &values));
    },
    None => panic!("the memo forgot the values it was given"),
  }
}

#[test]
fn the_default_marker_values_recorded_twice_keep_the_second() {
  let mut state = state();

  let first = compiled_styles("$$css", FlatCompiledStylesValue::Bool(true));
  let second = compiled_styles("color", FlatCompiledStylesValue::String("xred".to_string()));

  state.insert_cached_default_marker_values(first);
  state.insert_cached_default_marker_values(Rc::clone(&second));

  match state.cached_default_marker_values() {
    Some(read_back) => assert!(Rc::ptr_eq(read_back, &second)),
    None => panic!("the memo forgot the values it was given"),
  }
}

#[test]
fn a_rule_call_function_map_is_built_once_and_kept() {
  let mut state = state();

  assert!(
    state
      .cached_rule_call_function_map(RuleCallHelpers::FirstThatWorks)
      .is_none()
  );

  let map = Rc::new(FunctionMap::default());

  state.insert_cached_rule_call_function_map(RuleCallHelpers::FirstThatWorks, Rc::clone(&map));

  match state.cached_rule_call_function_map(RuleCallHelpers::FirstThatWorks) {
    Some(read_back) => assert!(Rc::ptr_eq(read_back, &map)),
    None => panic!("the memo forgot the map it was given"),
  }
}

#[test]
fn the_two_helper_sets_are_kept_apart() {
  let mut state = state();

  state.insert_cached_rule_call_function_map(
    RuleCallHelpers::FirstThatWorks,
    Rc::new(FunctionMap::default()),
  );

  assert!(
    state
      .cached_rule_call_function_map(RuleCallHelpers::FirstThatWorksAndKeyframes)
      .is_none()
  );

  let with_keyframes = Rc::new(FunctionMap::default());

  state.insert_cached_rule_call_function_map(
    RuleCallHelpers::FirstThatWorksAndKeyframes,
    Rc::clone(&with_keyframes),
  );

  match state.cached_rule_call_function_map(RuleCallHelpers::FirstThatWorksAndKeyframes) {
    Some(read_back) => assert!(Rc::ptr_eq(read_back, &with_keyframes)),
    None => panic!("the memo forgot the map it was given"),
  }
}

#[test]
fn a_rule_call_function_map_recorded_twice_keeps_the_second() {
  let mut state = state();

  let first = Rc::new(FunctionMap::default());
  let second = Rc::new(FunctionMap::default());

  state.insert_cached_rule_call_function_map(RuleCallHelpers::FirstThatWorks, first);
  state.insert_cached_rule_call_function_map(RuleCallHelpers::FirstThatWorks, Rc::clone(&second));

  match state.cached_rule_call_function_map(RuleCallHelpers::FirstThatWorks) {
    Some(read_back) => assert!(Rc::ptr_eq(read_back, &second)),
    None => panic!("the memo forgot the map it was given"),
  }
}
