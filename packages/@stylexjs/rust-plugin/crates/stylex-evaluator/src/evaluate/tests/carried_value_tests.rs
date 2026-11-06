//! A value the guard resolved, carried into the engine beside the printed
//! source.
//!
//! A name the fold resolves becomes a parameter of the printed arrow and its
//! value an argument to it, so the value travels *beside* the text rather than
//! inside it. Every shape the evaluator holds has to cross, and cross as the
//! thing it stands for -- a number as a number, an absent element as
//! `undefined`, an object as an object with the same keys -- because what runs
//! on the other side is JavaScript and reads them as such.
//!
//! Each case is written as a module binding and a call on the name, because
//! that is what makes the guard resolve anything at all: an expression written
//! out whole resolves no name and carries nothing.
//!
//! The two ceilings are here too. They are measured on the way in, before
//! anything is printed or built, so a value past one refuses rather than being
//! copied and then rejected.

use super::source_evaluation::*;
use stylex_constants::constants::evaluation_errors::{
  bound_value_has_too_many_entries, bound_value_too_large,
};
use stylex_structures::stylex_options::StyleXOptions;

/// The name every case binds its value to.
const CARRIED: &str = "carried";

/// What `source` folds to with `init` bound to the carried name.
#[track_caller]
fn folded_carrying(init: &str, source: &str) -> String {
  folded_in_a_module_binding(CARRIED, init, source)
}

/// Every primitive the bridge carries, read back through a method that can only
/// answer if the value arrived as what it stands for.
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

/// `null` and `undefined` cross as themselves, which `join` then writes as the
/// empty string -- the language's own answer, and the one that would be lost if
/// either had crossed as a string spelling its name.
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

/// A nested array crosses as a nested array, so a method that reaches inside it
/// finds the same shape the source wrote.
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

/// An object crosses with its own keys, in the order the source wrote them.
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

/// An object written with `__proto__` as a key sets the prototype rather than
/// carrying a property of that name, so the carried object holds no such own
/// key -- as the language says, and as the same object written out in the
/// printed text would behave.
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

/// A value whose text passes the character ceiling refuses on the way in, and
/// the refusal names the binding rather than the method -- the name is what an
/// author can shorten.
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

/// A value with more elements or properties than the ceiling allows refuses the
/// same way, and says which of the two bounds it passed.
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

/// A value inside the ceilings still folds, which is what keeps the two cases
/// above from passing against a bridge that carries nothing at all.
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

/// A key crosses as the string the language reads it as, whichever of the three
/// spellings the author wrote it in. The same reader names the property on both
/// sides, so a spelling read one way here and another there is how one property
/// comes to be two.
#[test]
fn a_key_crosses_as_the_name_the_language_reads_it_as() {
  assert_eq!(folded_carrying("{ ab: 1 }", "carried.ab + ''"), "1");
  assert_eq!(folded_carrying("{ 'a-b': 1 }", "carried['a-b'] + ''"), "1");
  assert_eq!(folded_carrying("{ 2: 3 }", "carried[2] + ''"), "3");
}

/// A key the bridge carries no form of stops the whole crossing, so the call is
/// handed back rather than folded against an object missing a property the
/// source wrote. A big integer is such a key: it is not a value this bridge
/// carries in any position.
///
/// Handed back is not the same as refused, and only the pair says which
/// happened: the same call over a key the bridge does carry folds, so the
/// refusal below belongs to the key rather than to the shape of the call.
#[test]
fn a_key_with_no_carried_form_is_handed_back() {
  let source = "[{ 1n: 1 }].map((entry) => entry)";

  assert_refused(&evaluate_source(source), source);

  assert_folds_to_number("[{ 1: 1 }].map((entry) => entry)[0][1]", 1.0);
}

/// A value of the compiler's own has no JavaScript form at all, so it crosses
/// as nothing and the call around it is handed back to the evaluator. That
/// hand-back is what makes the conversions and the callee shapes written out in
/// Rust reachable in the first place.
#[test]
fn a_value_with_no_javascript_form_is_handed_back() {
  let fns = a_function_fold();

  for source in [
    format!("[{FOLD_FUNCTION}].map((entry) => entry)"),
    format!("[{FOLD_NAMESPACE}].map((entry) => entry)"),
  ] {
    assert_refused(&evaluated_against(&fns, &source), &source);
  }

  // The same call over a value the bridge does carry folds, so what stopped
  // the two above is the value rather than the call around it.
  let source = "['a'].map((entry) => entry).join('')";

  assert_eq!(folded_text_of(evaluated_against(&fns, source), source), "a");
}

/// A name the expression reads twice crosses once. One parameter per name is
/// what the printed arrow needs -- a repeated parameter is a syntax error --
/// and the second reading answers the same value, so dropping it loses nothing.
#[test]
fn a_name_read_twice_crosses_once() {
  assert_eq!(
    folded_carrying("['a', 'b']", "carried.concat(carried).join('-')"),
    "a-b-a-b"
  );
}

/// A string the language admits and Rust does not crosses exactly. A JavaScript
/// string literal can hold an unpaired surrogate, so the value is carried as the
/// code units it is written from rather than through a Rust string it has no
/// valid reading as.
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

/// A bare primitive crosses on its own, not only as an element of a list.
///
/// Every case above carries a *container* -- an array or an object -- so the
/// bridge reached each primitive through one. A name bound straight to a
/// number, a boolean or either absent value is the other half, and it is the
/// half a method called on the name itself reads.
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
