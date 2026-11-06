//! The module-level names the compiler generates for the expressions it lifts
//! out of a `stylex.create` call.
//!
//! Two properties matter. The name is a function of the file alone, so the same
//! input always gives the same output whatever the compiler did before it. And
//! the name is free, so it cannot take a name the module already uses.

use crate::tests::prelude::test_state as state;

#[test]
fn gives_the_first_name_without_a_number() {
  let mut state = state();

  assert_eq!(state.next_hoisted_ident("temp").sym.as_str(), "_temp");
  assert_eq!(state.next_hoisted_ident("temp").sym.as_str(), "_temp2");
  assert_eq!(state.next_hoisted_ident("temp").sym.as_str(), "_temp3");
}

#[test]
fn counts_each_stem_on_its_own() {
  let mut state = state();

  assert_eq!(state.next_hoisted_ident("temp").sym.as_str(), "_temp");
  assert_eq!(state.next_hoisted_ident("styles").sym.as_str(), "_styles");
  assert_eq!(state.next_hoisted_ident("temp").sym.as_str(), "_temp2");
  assert_eq!(state.next_hoisted_ident("styles").sym.as_str(), "_styles2");
}

/// The property the whole change is for. One file must not see the names
/// another file took, or the output of a file depends on the order the compiler
/// read the files in.
#[test]
fn starts_again_for_the_next_file() {
  let mut first = state();
  let mut second = state();

  for _ in 0..40 {
    first.next_hoisted_ident("temp");
  }

  assert_eq!(second.next_hoisted_ident("temp").sym.as_str(), "_temp");
}

#[test]
fn passes_over_a_name_the_module_already_uses() {
  let mut state = state();
  state.bound_names.insert("_temp".to_string());

  assert_eq!(state.next_hoisted_ident("temp").sym.as_str(), "_temp2");
}

#[test]
fn passes_over_a_run_of_names_the_module_already_uses() {
  let mut state = state();

  for name in ["_temp", "_temp2", "_temp3"] {
    state.bound_names.insert(name.to_string());
  }

  assert_eq!(state.next_hoisted_ident("temp").sym.as_str(), "_temp4");
  assert_eq!(state.next_hoisted_ident("temp").sym.as_str(), "_temp5");
}

/// A name taken in the middle of the run is passed over, and the count goes on
/// from where it stopped.
#[test]
fn passes_over_a_name_taken_in_the_middle_of_the_run() {
  let mut state = state();
  state.bound_names.insert("_temp2".to_string());

  assert_eq!(state.next_hoisted_ident("temp").sym.as_str(), "_temp");
  assert_eq!(state.next_hoisted_ident("temp").sym.as_str(), "_temp3");
}

/// A module can hold a great many hoisted expressions. The names stay unique
/// and in order.
#[test]
fn gives_a_unique_name_for_every_one_of_many_expressions() {
  let mut state = state();

  let names: Vec<String> = (0..5_000)
    .map(|_| state.next_hoisted_ident("temp").sym.to_string())
    .collect();

  let unique: std::collections::BTreeSet<&String> = names.iter().collect();

  assert_eq!(unique.len(), names.len());
  assert_eq!(names[0], "_temp");
  assert_eq!(names[4_999], "_temp5000");
}

/// The name goes on a `const` declaration of the module, so it is a binding
/// like any other and every later question about the name sees it.
#[test]
fn records_the_name_it_gave_as_a_binding() {
  let mut state = state();
  let name = state.next_hoisted_ident("temp").sym.to_string();

  assert_eq!(name, "_temp");
  assert!(state.has_binding("_temp"));
}
