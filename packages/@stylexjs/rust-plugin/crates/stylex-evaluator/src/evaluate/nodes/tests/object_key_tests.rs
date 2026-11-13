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
  ecma::ast::{BigInt, ComputedPropName, Expr, KeyValueProp, Lit, PropName, Str},
};

use stylex_ast::ast::convertors::{convert_atom_to_string, create_number_expr};

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
