//! The answers the state works out once for a file and reads back.
//!
//! Each of these is a memo, not a question about the module: the answer is the
//! same for every call in one file, and working it out again would mean reading
//! the package boundaries around the file, or building the same two strings and
//! the same index map. A memo owes three answers -- nothing before it is
//! written, the value it was given after, and the second value when it is
//! written twice -- so each one is asked all three.

use std::rc::Rc;

use crate::flat_compiled_styles_value::FlatCompiledStylesValue;
use crate::functions::{FunctionMap, RuleCallHelpers};
use crate::tests::prelude::test_state as state;
use crate::types::FlatCompiledStyles;

/// One compiled style map holding the single entry handed in, which is all any
/// case here reads back. It is shared, because the memo keeps it shared.
fn compiled_styles(key: &str, value: FlatCompiledStylesValue) -> Rc<FlatCompiledStyles> {
  let mut values = FlatCompiledStyles::default();

  values.insert(key.to_string(), Rc::new(value));

  Rc::new(values)
}

/// The short filename a debug annotation carries is worked out once and kept,
/// because reading it means reading the package boundaries around the file.
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

/// A path recorded twice keeps the second answer, which is what an overwrite
/// has to do for the memo to stay a memo of the current shortening.
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

/// The default marker's compiled values are the same for every call in one
/// file, so they are built once and read back. An empty memo answers nothing,
/// which is what makes the first call build them.
#[test]
fn the_default_marker_values_are_built_once_and_kept() {
  let mut state = state();

  assert_eq!(state.cached_default_marker_values(), None);

  let values = compiled_styles("$$css", FlatCompiledStylesValue::Bool(true));

  state.insert_cached_default_marker_values(Rc::clone(&values));

  match state.cached_default_marker_values() {
    Some(read_back) => {
      // The same object, not an equal one. Every call in the file registers the
      // marker again, so a memo that copied it would cost what it saves.
      assert!(Rc::ptr_eq(read_back, &values));
    },
    None => panic!("the memo forgot the values it was given"),
  }
}

/// A second build overwrites the first. Nothing in a file asks for two markers,
/// but the memo must answer the values it was last given rather than the values
/// it was first given.
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

/// One function map per set of helpers, built by the first rule call of
/// the file and read back by the rest. An empty memo answers nothing, which is
/// what makes the first call build one.
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
    // The same object, not an equal one. The map is read by every later call,
    // and a memo that copied it would cost what it saves.
    Some(read_back) => assert!(Rc::ptr_eq(read_back, &map)),
    None => panic!("the memo forgot the map it was given"),
  }
}

/// The two sets are two memos. `viewTransitionClass` registers `keyframes` and
/// the other two calls must not see it, so a map kept under one set is no
/// answer for the other.
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

/// A set recorded twice keeps the second map, which is what an overwrite has to
/// do for the memo to stay a memo of the current registration.
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
