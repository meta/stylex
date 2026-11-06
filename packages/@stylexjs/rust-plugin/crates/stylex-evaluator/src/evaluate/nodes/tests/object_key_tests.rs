//! The string an object key names, whichever way it was written.
//!
//! `evaluate_obj_key` is what the transform above asks for the *name* of a
//! property, and every spelling a key can have has to answer the same string
//! the language names the property with -- a key read one way here and another
//! where the object itself is folded is how one property becomes two rules.
//!
//! Asked directly rather than through a folded object, because this is the
//! entry point the transform calls and its refusals are answers rather than
//! deopts recorded on a state.

use crate::evaluate::evaluate_obj_key;
use crate::evaluate::source_evaluation::*;
use stylex_constants::constants::messages::KEY_HAS_NO_NAME;
use stylex_state::{
  evaluate_result_value::EvaluateResultValue, functions::FunctionMap, state_manager::StateManager,
};
use stylex_structures::stylex_options::StyleXOptions;
use swc_core::atoms::Wtf8Atom;
use swc_core::atoms::wtf8::{CodePoint, Wtf8Buf};
use swc_core::{
  common::{DUMMY_SP, GLOBALS, Globals},
  ecma::ast::{BigInt, ComputedPropName, Expr, IdentName, KeyValueProp, Lit, PropName, Str},
};

use stylex_ast::ast::convertors::{convert_atom_to_string, create_number_expr, create_string_expr};

/// A string literal holding one lone surrogate, which no Rust `str` can spell.
///
/// The one expression with a form and no text, which is what separates the two
/// ways a key can have no name.
fn lone_surrogate_expr() -> Expr {
  let mut buffer = Wtf8Buf::new();

  match CodePoint::from_u32(0xD83D) {
    Some(point) => buffer.push(point),
    None => panic!("the high surrogate is a code point"),
  }

  Expr::Lit(Lit::Str(Str {
    span: DUMMY_SP,
    value: Wtf8Atom::from(buffer),
    raw: None,
  }))
}

/// The key `name` names, or the refusal it answered.
fn key_of(name: PropName) -> Result<String, Option<String>> {
  let globals = Globals::new();

  GLOBALS.set(&globals, || {
    let mut state = StateManager::new(StyleXOptions::default());
    let result = evaluate_obj_key(
      &KeyValueProp {
        key: name,
        value: Box::new(create_number_expr(1.0)),
      },
      &mut state,
      &FunctionMap::default(),
    );

    match (result.confident, result.value) {
      (true, Some(EvaluateResultValue::Expr(Expr::Lit(literal)))) => match literal.as_str() {
        Some(text) => Ok(convert_atom_to_string(&text.value)),
        None => panic!("expected a string key, got {:?}", literal),
      },
      (true, other) => panic!("expected a string key, got {:?}", other),
      (false, _) => Err(result.reason),
    }
  })
}

/// A key written as a big-integer literal, which has no other spelling.
fn big_int_key(digits: u64) -> PropName {
  PropName::BigInt(BigInt {
    span: DUMMY_SP,
    value: Box::new(digits.into()),
    raw: None,
  })
}

fn computed(expr: Expr) -> PropName {
  PropName::Computed(ComputedPropName {
    span: DUMMY_SP,
    expr: Box::new(expr),
  })
}

/// The three spellings that name a key without evaluating anything: an
/// identifier, a string, and a number. The number is rendered as JavaScript
/// spells it rather than as Rust does.
#[test]
fn a_written_key_names_itself() {
  assert_eq!(
    key_of(PropName::Ident(IdentName::new("color".into(), DUMMY_SP))),
    Ok(String::from("color"))
  );
  assert_eq!(
    key_of(PropName::Str("font-size".into())),
    Ok(String::from("font-size"))
  );
  assert_eq!(
    key_of(PropName::Num(1e21.into())),
    Ok(String::from("1e+21"))
  );
}

/// A computed key is the string its expression folds to.
#[test]
fn a_computed_key_is_what_it_folds_to() {
  assert_eq!(
    key_of(computed(create_string_expr("color"))),
    Ok(String::from("color"))
  );
  assert_eq!(
    key_of(computed(create_number_expr(2.0))),
    Ok(String::from("2"))
  );
}

/// The three numbers with no digits of their own name themselves, and negative
/// zero names the same property positive zero does.
///
/// Each is a number `f64::to_string` spells differently from the language:
/// `NaN` and `inf` rather than `NaN` and `Infinity`, and `-0` rather than `0`.
/// A key is a CSS property name, so a spelling of the compiler's own would
/// declare a property no browser was asked for.
#[test]
fn the_numbers_with_no_digits_name_themselves() {
  for (value, expected) in [
    (f64::NAN, "NaN"),
    (f64::INFINITY, "Infinity"),
    (f64::NEG_INFINITY, "-Infinity"),
    (-0.0, "0"),
    (0.0, "0"),
  ] {
    assert_eq!(
      key_of(computed(create_number_expr(value))),
      Ok(String::from(expected)),
      "wrong key for the number {}",
      expected
    );
  }
}

/// A computed key whose expression resolves to nothing carries that
/// expression's own refusal, so the sentence names what the author wrote.
#[test]
fn a_computed_key_that_resolves_to_nothing_carries_its_own_refusal() {
  let refusal = key_of(computed(parse_expr("unknownName")));

  assert!(
    matches!(&refusal, Err(Some(reason)) if reason.contains("not defined")),
    "expected the name's own refusal, got {:?}",
    refusal
  );
}

/// A key that folded to the evaluator's own list is read through the array it
/// writes, so it names the property its elements join to -- which is what the
/// language and the reference implementation both name it.
#[test]
fn a_computed_key_that_folded_to_a_list_names_its_joined_elements() {
  assert_eq!(
    key_of(computed(parse_expr("[1, 2]"))),
    Ok(String::from("1,2"))
  );
  assert_eq!(
    key_of(computed(parse_expr("[[1], [2, 3]]"))),
    Ok(String::from("1,2,3"))
  );
}

/// The object fold reads a computed key exactly as `evaluate_obj_key` does, so
/// an object literal written as one names `[object Object]` in both places.
///
/// The two used to answer one mistake with two sentences -- `The key is not a
/// string.` here and `Expected a string value but received a non-string
/// expression.` inside a folded object.
#[test]
fn the_object_fold_reads_a_computed_key_the_same_way() {
  assert_folds_to_object_keys("({ [{}]: 'x' })", &["[object Object]"]);
  assert_folds_to_object_keys("({ [true]: 'x' })", &["true"]);
  assert_folds_to_object_keys("({ [1 > 2]: 'x' })", &["false"]);
}

/// A key written as a big-integer literal names its digits, which is what the
/// language names the property: `1n` and `1` are the same key. Pinned against
/// the reference implementation, which declares `1: red` for `{ 1n: 'red' }`.
#[test]
fn a_big_integer_key_names_its_digits() {
  assert_eq!(key_of(big_int_key(1)), Ok(String::from("1")));
  assert_eq!(key_of(big_int_key(0)), Ok(String::from("0")));
  assert_eq!(
    key_of(big_int_key(u64::MAX)),
    Ok(String::from("18446744073709551615")),
    "a value past what a number holds is still written out in full"
  );
}

/// The same key written in source, so the spelling and the entry point agree.
#[test]
fn a_big_integer_key_written_in_source_names_the_same_string() {
  assert_folds_to_object_keys("({ 1n: 'x' })", &["1"]);
  // An integer key is ordered ahead of a written one, which is the order the
  // language gives own keys and the order the reference implementation emits
  // the two rules in.
  assert_folds_to_object_keys("({ color: 'red', 2n: 'blue' })", &["2", "color"]);
}

/// A computed key names the property `String(key)`, which is what the language
/// names it and what the reference implementation writes.
///
/// It refused every one of these before, as a key with no name. None
/// of them is a CSS property anybody writes on purpose, but the refusal was
/// inherited rather than decided: the key was read by the converter that spells
/// a string, which answers for a string and a number and nothing else.
///
/// Each row is measured against `@stylexjs/babel-plugin` 0.19.0.
#[test]
fn a_computed_key_names_the_string_the_language_names_it() {
  for (source, expected) in [
    ("true", "true"),
    ("false", "false"),
    ("null", "null"),
    ("undefined", "undefined"),
    ("!1", "false"),
    ("({})", "[object Object]"),
    ("({ a: 1 })", "[object Object]"),
    ("[1, 2]", "1,2"),
    ("[]", ""),
    ("'color'", "color"),
    ("2", "2"),
    ("NaN", "NaN"),
    ("Infinity", "Infinity"),
    ("-0", "0"),
  ] {
    assert_eq!(
      key_of(computed(parse_expr(source))),
      Ok(String::from(expected)),
      "wrong key for the computed key `{}`",
      source
    );
  }
}

/// A comparison names the property its boolean spells, which is the word rather
/// than the digit.
///
/// It named `0` and `1` before: a comparison folded through the numeric reading
/// of a binary expression, so the two compilers wrote two different property
/// names for one source with no error either side.
#[test]
fn a_comparison_read_as_a_key_names_the_word_it_folds_to() {
  assert_eq!(
    key_of(computed(parse_expr("1 > 2"))),
    Ok(String::from("false"))
  );
  assert_eq!(
    key_of(computed(parse_expr("2 > 1"))),
    Ok(String::from("true"))
  );
}

/// A key with no compile-time string at all still refuses, and still says so as
/// a key. A function is the one such value: its `String` is its source text,
/// which this evaluator does not keep.
#[test]
fn a_computed_key_with_no_string_at_all_refuses_as_a_key() {
  // A function, whose `String` is its source text. It has no expression form
  // here either, so the value is what fails to read.
  assert_eq!(
    key_of(computed(parse_expr("(() => 1)"))),
    Err(Some(KEY_HAS_NO_NAME.to_string()))
  );

  // A text holding half of an astral character. It *is* an expression, and the
  // coercion is what has no string for it: no Rust string holds a lone
  // surrogate.
  assert_eq!(
    key_of(computed(lone_surrogate_expr())),
    Err(Some(KEY_HAS_NO_NAME.to_string()))
  );

  // The same text written as the key rather than computed into it. It used to
  // abort the build from inside the converter that spells an atom; it now
  // refuses beside every other key that has no name, so all three ways a key
  // can lack one end under one sentence.
  let lone_surrogate = match lone_surrogate_expr() {
    Expr::Lit(Lit::Str(text)) => text,
    other => panic!("expected a string literal, got {:?}", other),
  };

  assert_eq!(
    key_of(PropName::Str(lone_surrogate)),
    Err(Some(KEY_HAS_NO_NAME.to_string()))
  );
}

// ==================== a key written twice ====================

/// A key written twice is one property carrying the later value, however the
/// second spelling arrived.
///
/// Three routes reach the same set -- a second literal property, a spread that
/// carries the key, and a computed key that folds to it -- and the fold reads
/// them at three different moments. The language answers all three the same
/// way, so a route that kept both would write a property the source does not
/// describe, and a route that kept the first value would write the wrong one.
#[test]
fn a_key_written_twice_keeps_one_property_and_the_later_value() {
  for source in [
    "({ a: 1, a: 2 })",
    "({ a: 1, ...{ a: 2 } })",
    "({ a: 1, ['a']: 2 })",
  ] {
    assert_folds_to_object_keys(source, &["a"]);
    assert_folds_to_number(&format!("({source}).a"), 2.0);
  }
}

/// The empty string is a key like any other, and an array-index key sorts ahead
/// of it -- which is the order the language answers and the order a class name
/// is hashed from.
#[test]
fn the_empty_string_is_a_key_and_an_index_sorts_ahead_of_it() {
  assert_folds_to_object_keys("({ '': 1, 0: 2 })", &["0", ""]);
  assert_folds_to_object_keys("({ 0: 1, '': 2, a: 3 })", &["0", "", "a"]);

  assert_folds_to_number("({ '': 1, 0: 2 })['']", 1.0);
  assert_folds_to_number("({ '': 1, 0: 2 })[0]", 2.0);
}
