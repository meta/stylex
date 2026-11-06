//! Tests object-key coercion, refusals and duplicate-key handling.

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

/// Builds a lone-surrogate literal that Rust `str` cannot represent.
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

#[test]
fn a_computed_key_that_resolves_to_nothing_carries_its_own_refusal() {
  let refusal = key_of(computed(parse_expr("unknownName")));

  assert!(
    matches!(&refusal, Err(Some(reason)) if reason.contains("not defined")),
    "expected the name's own refusal, got {:?}",
    refusal
  );
}

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

#[test]
fn the_object_fold_reads_a_computed_key_the_same_way() {
  assert_folds_to_object_keys("({ [{}]: 'x' })", &["[object Object]"]);
  assert_folds_to_object_keys("({ [true]: 'x' })", &["true"]);
  assert_folds_to_object_keys("({ [1 > 2]: 'x' })", &["false"]);
}

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

#[test]
fn a_big_integer_key_written_in_source_names_the_same_string() {
  assert_folds_to_object_keys("({ 1n: 'x' })", &["1"]);
  assert_folds_to_object_keys("({ color: 'red', 2n: 'blue' })", &["2", "color"]);
}

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

#[test]
fn a_computed_key_with_no_string_at_all_refuses_as_a_key() {
  assert_eq!(
    key_of(computed(parse_expr("(() => 1)"))),
    Err(Some(KEY_HAS_NO_NAME.to_string()))
  );

  assert_eq!(
    key_of(computed(lone_surrogate_expr())),
    Err(Some(KEY_HAS_NO_NAME.to_string()))
  );

  let lone_surrogate = match lone_surrogate_expr() {
    Expr::Lit(Lit::Str(text)) => text,
    other => panic!("expected a string literal, got {:?}", other),
  };

  assert_eq!(
    key_of(PropName::Str(lone_surrogate)),
    Err(Some(KEY_HAS_NO_NAME.to_string()))
  );
}

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

#[test]
fn the_empty_string_is_a_key_and_an_index_sorts_ahead_of_it() {
  assert_folds_to_object_keys("({ '': 1, 0: 2 })", &["0", ""]);
  assert_folds_to_object_keys("({ 0: 1, '': 2, a: 3 })", &["0", "", "a"]);

  assert_folds_to_number("({ '': 1, 0: 2 })['']", 1.0);
  assert_folds_to_number("({ '': 1, 0: 2 })[0]", 2.0);
}
