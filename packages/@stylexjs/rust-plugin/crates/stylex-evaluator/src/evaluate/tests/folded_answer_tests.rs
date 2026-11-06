//! What the engine answered, read back as a value the evaluator carries.
//!
//! The two allocation ceilings are the subject. The engine aliases, so one
//! string referenced a thousand times costs it one string; this side copies, so
//! the same answer is a thousand strings and a syntax node each. Nothing on the
//! way in bounds that, because nothing on the way in wrote it: the source of
//! `['abcd', 'efgh'].map((s) => s)` names no value at all, and the two strings
//! are the engine's own. So the counting is here, and it runs as the answer is
//! read.
//!
//! Beside them sit the two readings that need the engine while the answer is
//! being read: a property whose getter runs, and a value this side writes no
//! expression for.
//!
//! Every case names the number it is about rather than building an answer large
//! enough to pass the shipped one, and every refusal is paired with the same
//! shape inside the bound -- a ceiling case that refuses at every size says
//! nothing about the ceiling.

use super::source_evaluation::*;
use crate::evaluate_result::EvaluateResult;
use stylex_ast::ast::convertors::convert_atom_to_string;
use stylex_constants::constants::evaluation_errors::{
  folded_string_too_large, object_size_too_large, unfoldable_fold_result, unfoldable_static,
};
use stylex_state::{evaluate_result_value::EvaluateResultValue, functions::FunctionMap};
use swc_core::ecma::ast::{Expr, Lit, Prop, PropOrSpread};

/// Evaluates `source` under the character ceiling `limit` and nothing else.
///
/// Every case here folds a written-out expression, so no case needs a function
/// map and one default serves them all.
fn at_a_character_ceiling(limit: usize, source: &str) -> Box<EvaluateResult> {
  evaluated_at_a_character_ceiling(limit, &FunctionMap::default(), source)
}

/// The same, under the entry ceiling.
fn at_an_entry_ceiling(limit: usize, source: &str) -> Box<EvaluateResult> {
  evaluated_at_an_entry_ceiling(limit, &FunctionMap::default(), source)
}

/// The strings an array answer holds, in order.
#[track_caller]
fn strings_of(value: EvaluateResultValue, source: &str) -> Vec<String> {
  match value {
    EvaluateResultValue::Vec(items) => items
      .iter()
      .map(|item| match item {
        EvaluateResultValue::Expr(expr) => string_of(expr, source),
        other => panic!("expected `{}` to hold strings, got {:?}", source, other),
      })
      .collect(),
    other => panic!("expected `{}` to fold to an array, got {:?}", source, other),
  }
}

/// The own keys of an object answer, in order.
#[track_caller]
fn keys_of(value: EvaluateResultValue, source: &str) -> Vec<String> {
  properties_of(value, source)
    .into_iter()
    .map(|(key, _)| key)
    .collect()
}

/// The own keys of an object answer paired with the strings under them.
///
/// The pair rather than the keys alone: a key and a value are counted through
/// one total, so a case about that total has to be able to say which of the two
/// arrived.
#[track_caller]
fn entries_of(value: EvaluateResultValue, source: &str) -> Vec<(String, String)> {
  properties_of(value, source)
    .iter()
    .map(|(key, value)| (key.clone(), string_of(value, source)))
    .collect()
}

#[track_caller]
fn properties_of(value: EvaluateResultValue, source: &str) -> Vec<(String, Expr)> {
  let object = match value {
    EvaluateResultValue::Expr(Expr::Object(object)) => object,
    other => panic!(
      "expected `{}` to fold to an object, got {:?}",
      source, other
    ),
  };

  object
    .props
    .iter()
    .map(|prop| match prop {
      PropOrSpread::Prop(prop) => match prop.as_ref() {
        Prop::KeyValue(pair) => match pair.key.as_ident() {
          Some(name) => (name.sym.to_string(), *pair.value.clone()),
          None => panic!("expected `{}` to fold to identifier keys", source),
        },
        other => panic!(
          "expected `{}` to fold to key-value props, got {:?}",
          source, other
        ),
      },
      PropOrSpread::Spread(_) => {
        panic!("expected `{}` to fold to an object with no spread", source)
      },
    })
    .collect()
}

#[track_caller]
fn string_of(expr: &Expr, source: &str) -> String {
  match expr {
    Expr::Lit(Lit::Str(text)) => convert_atom_to_string(&text.value),
    other => panic!("expected `{}` to hold a string, got {:?}", source, other),
  }
}

/// A string the engine built is counted on the way back, and the refusal names
/// the ceiling it passed.
///
/// The receiver is written out rather than named, so nothing crosses on the way
/// in and all eight characters are the answer's own.
#[test]
fn a_string_the_answer_carries_is_counted_against_the_ceiling() {
  let source = "'abcdefgh'.toUpperCase()";

  assert_refused_with(
    &at_a_character_ceiling(4, source),
    source,
    &folded_string_too_large(4),
  );

  assert_eq!(
    folded_text_of(at_a_character_ceiling(8, source), source),
    "ABCDEFGH"
  );
}

/// Every string of one answer is counted against one running total, rather than
/// each being measured on its own.
///
/// Four characters each and seven allowed: a check per value waves both of them
/// through, and the total is what refuses the pair. The same pair at eight
/// folds, which is what says the number being read is the sum.
#[test]
fn every_string_of_one_answer_is_counted_against_the_same_total() {
  let source = "['abcd', 'efgh'].map((s) => s)";

  assert_refused_with(
    &at_a_character_ceiling(7, source),
    source,
    &folded_string_too_large(7),
  );

  assert_eq!(
    strings_of(
      folded_value_of(at_a_character_ceiling(8, source), source),
      source
    ),
    ["abcd", "efgh"]
  );
}

/// A key is text the answer holds as surely as a value is, so it is counted
/// against the same ceiling.
///
/// The only string here is the key -- the value is a number, which costs no
/// characters at all -- so a ceiling the key alone passes is what tells this
/// reading from the one above it.
#[test]
fn a_key_of_a_folded_object_is_counted_as_a_string() {
  let source = "Object.fromEntries([['abcdefgh', 1]])";

  assert_refused_with(
    &at_a_character_ceiling(4, source),
    source,
    &folded_string_too_large(4),
  );

  assert_eq!(
    keys_of(
      folded_value_of(at_a_character_ceiling(8, source), source),
      source
    ),
    ["abcdefgh"]
  );
}

/// A key and a value share that one total, so an object of two short strings is
/// bounded by their sum rather than by the longer of them.
#[test]
fn a_key_and_a_value_share_the_one_total() {
  let source = "Object.fromEntries([['abcd', 'efgh']])";

  assert_refused_with(
    &at_a_character_ceiling(7, source),
    source,
    &folded_string_too_large(7),
  );

  assert_eq!(
    entries_of(
      folded_value_of(at_a_character_ceiling(8, source), source),
      source
    ),
    [("abcd".to_string(), "efgh".to_string())]
  );
}

/// An object with more properties than the entry ceiling refuses on its own
/// count, before a single property of it is read back.
///
/// Its own sentence rather than the array's: the two are bounded by the same
/// number and are different things for an author to shorten.
#[test]
fn an_object_past_the_entry_ceiling_refuses_by_its_property_count() {
  let source = "Object.fromEntries([['a', 1], ['b', 2], ['c', 3]])";

  assert_refused_with(
    &at_an_entry_ceiling(2, source),
    source,
    &object_size_too_large(2),
  );

  assert_eq!(
    keys_of(
      folded_value_of(at_an_entry_ceiling(3, source), source),
      source
    ),
    ["a", "b", "c"]
  );
}

/// A reflective static refuses before the engine is reached.
///
/// `Object.create` and the rest of them hand back an object from the prototype
/// chain, which is a step from a plain object to `Function`. The allowlist does
/// not hold them, so they read the one sentence every refused static reads.
///
/// The read-back throw this case once reached is now asked of the walk out
/// directly, in `built_answer_tests`: no source builds a getter any more.
///
/// Every one of them takes an object first and reads a key second, so one call
/// shape reaches them all.
#[test]
fn a_reflective_static_refuses_before_the_engine() {
  for method in [
    "create",
    "getPrototypeOf",
    "setPrototypeOf",
    "getOwnPropertyDescriptor",
    "getOwnPropertyDescriptors",
  ] {
    let source = format!("Object.{}({{ a: 1 }}, 'a')", method);
    let result = evaluate_source(&source);

    assert_refused(&result, &source);
    assert_eq!(
      result.reason.as_deref(),
      Some(unfoldable_static("Object", method).as_str()),
      "the refusal for `{}`",
      source
    );
  }
}

/// A function under a key has no expression this side writes, so the whole
/// answer refuses and names the kind the language answered with.
#[test]
fn a_function_under_a_key_refuses_by_the_kind_it_is() {
  let source = "Object.fromEntries([['a', () => 1]])";

  assert_refused_with(
    &evaluate_source(source),
    source,
    &unfoldable_fold_result("function"),
  );
}
