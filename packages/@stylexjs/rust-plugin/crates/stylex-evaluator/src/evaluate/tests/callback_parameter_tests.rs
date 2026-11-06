//! What a callback's parameter list binds, as the guard walks it.
//!
//! A callback that runs inside the fold is JavaScript the engine executes, so
//! its parameters bind whatever the language says they bind. What the guard has
//! to do first is *read* the list: every name a pattern introduces shadows a
//! name the fold would otherwise carry, and every expression written inside one
//! -- a default, a computed key -- is an expression the guard has to admit
//! before anything is printed.
//!
//! A pattern the guard read wrongly does not fail loudly. It carries a value
//! under a name the callback rebinds, or prints an expression it never
//! admitted, so every shape a parameter list can take is walked here.
//!
//! Each case is written against a module binding, because a callback the guard
//! never resolved a name for is a callback the fold does not reach.

use super::source_evaluation::*;

/// The name every case binds the list it maps over to.
const MAPPED: &str = "parts";

/// What `source` folds to with `init` bound to that name.
#[track_caller]
fn folded_mapping(init: &str, source: &str) -> String {
  folded_in_a_module_binding(MAPPED, init, source)
}

/// An array pattern binds each element by position, and nests.
#[test]
fn an_array_pattern_binds_by_position() {
  assert_eq!(
    folded_mapping(
      "[['a', 'b'], ['c', 'd']]",
      "parts.map(([first, second]) => first + second).join('-')"
    ),
    "ab-cd"
  );
  assert_eq!(
    folded_mapping(
      "[[1, [2]]]",
      "parts.map(([first, [inner]]) => first + inner).join('-')"
    ),
    "3"
  );
}

/// An object pattern binds by key, in all three spellings the language has --
/// the shorthand, the renaming one, and the computed one whose key is an
/// expression the guard has to admit before it can be printed.
#[test]
fn an_object_pattern_binds_by_key_in_every_spelling() {
  assert_eq!(
    folded_mapping(
      "[{ a: 'x' }, { a: 'y' }]",
      "parts.map(({ a }) => a).join('-')"
    ),
    "x-y"
  );
  assert_eq!(
    folded_mapping(
      "[{ a: 'x' }]",
      "parts.map(({ a: renamed }) => renamed).join('-')"
    ),
    "x"
  );
  assert_eq!(
    folded_mapping(
      "[{ a: 'x' }]",
      "parts.map(({ ['a']: computed }) => computed).join('-')"
    ),
    "x"
  );
}

/// A default is an expression written inside the parameter list, and it is the
/// one the callback answers when nothing was passed for it -- in both the
/// shorthand spelling and the plain one.
#[test]
fn a_default_in_a_parameter_list_is_the_value_a_missing_argument_takes() {
  assert_eq!(
    folded_mapping("[{}]", "parts.map(({ a = 'x' }) => a).join('-')"),
    "x"
  );
  assert_eq!(
    folded_mapping(
      "['a', undefined]",
      "parts.map((each = 'z') => each).join('-')"
    ),
    "a-z"
  );
}

/// A rest element gathers what the patterns before it did not take, in both the
/// parameter list and an object pattern inside one.
#[test]
fn a_rest_element_gathers_what_is_left() {
  assert_eq!(
    folded_mapping("['a']", "parts.map((...rest) => rest[0]).join('-')"),
    "a"
  );
  assert_eq!(
    folded_mapping(
      "[{ a: 'x' }]",
      "parts.map(({ ...rest }) => rest.a).join('-')"
    ),
    "x"
  );
}

/// A computed member read inside a callback is an expression the guard admits
/// like any other, so the key can be the parameter the callback binds.
#[test]
fn a_computed_read_inside_a_callback_is_admitted() {
  assert_eq!(
    folded_mapping(
      "['a', 'b']",
      "parts.map((key) => ({ a: 1, b: 2 })[key]).join('-')"
    ),
    "1-2"
  );
}
