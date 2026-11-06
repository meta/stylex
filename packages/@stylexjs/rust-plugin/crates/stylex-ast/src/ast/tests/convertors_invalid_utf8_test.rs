//! Panic-path coverage for convertor helpers that require valid UTF-8.

use crate::ast::convertors::*;
use swc_core::{atoms::Wtf8Atom, common::DUMMY_SP, ecma::ast::*};

fn invalid_wtf8_atom() -> Wtf8Atom {
  // Unpaired surrogate: valid WTF-8 storage, invalid UTF-8 decoding.
  unsafe { Wtf8Atom::from_bytes_unchecked(&[0xed, 0xa0, 0x80]) }
}

/// `convert_tpl_to_string_lit` should panic when cooked text is missing.
#[test]
fn convert_tpl_to_string_lit_with_missing_cooked_panics() {
  let tpl = Tpl {
    span: DUMMY_SP,
    exprs: vec![],
    quasis: vec![TplElement {
      span: DUMMY_SP,
      tail: true,
      cooked: None,
      raw: "".into(),
    }],
  };

  let result = std::panic::catch_unwind(|| convert_tpl_to_string_lit(&tpl));
  assert!(result.is_err());
}

/// `convert_tpl_to_string_lit` should panic when cooked text is invalid UTF-8.
#[test]
fn convert_tpl_to_string_lit_with_invalid_utf8_panics() {
  let tpl = Tpl {
    span: DUMMY_SP,
    exprs: vec![],
    quasis: vec![TplElement {
      span: DUMMY_SP,
      tail: true,
      cooked: Some(invalid_wtf8_atom()),
      raw: "".into(),
    }],
  };

  let result = std::panic::catch_unwind(|| convert_tpl_to_string_lit(&tpl));
  assert!(result.is_err());
}

/// Atom-to-string conversion should panic on invalid UTF-8 payloads.
#[test]
fn convert_atom_to_string_invalid_utf8_panics() {
  let atom = invalid_wtf8_atom();
  let result = std::panic::catch_unwind(|| convert_atom_to_string(&atom));
  assert!(result.is_err());
}

/// WTF-8 conversion should panic when UTF-8 decoding fails.
#[test]
fn convert_wtf8_to_atom_invalid_utf8_panics() {
  let atom = invalid_wtf8_atom();
  let result = std::panic::catch_unwind(|| convert_wtf8_to_atom(&atom));
  assert!(result.is_err());
}

/// String literal conversion should panic on invalid UTF-8 payloads.
#[test]
fn convert_str_lit_to_string_invalid_utf8_panics() {
  let str_lit = Str {
    span: DUMMY_SP,
    value: invalid_wtf8_atom(),
    raw: None,
  };
  let result = std::panic::catch_unwind(|| convert_str_lit_to_string(&str_lit));
  assert!(result.is_err());
}

/// Atom extraction should panic on invalid UTF-8 payloads.
#[test]
fn convert_str_lit_to_atom_invalid_utf8_panics() {
  let str_lit = Str {
    span: DUMMY_SP,
    value: invalid_wtf8_atom(),
    raw: None,
  };
  let result = std::panic::catch_unwind(|| convert_str_lit_to_atom(&str_lit));
  assert!(result.is_err());
}

/// Cooked template extraction should panic when bytes are invalid UTF-8.
#[test]
fn extract_tpl_cooked_value_invalid_utf8_panics() {
  let elem = TplElement {
    span: DUMMY_SP,
    tail: true,
    cooked: Some(invalid_wtf8_atom()),
    raw: "".into(),
  };
  let result = std::panic::catch_unwind(|| extract_tpl_cooked_value(&elem));
  assert!(result.is_err());
}

/// Cooked template extraction should panic when no cooked value exists.
#[test]
fn extract_tpl_cooked_value_missing_cooked_panics() {
  let elem = TplElement {
    span: DUMMY_SP,
    tail: true,
    cooked: None,
    raw: "".into(),
  };
  let result = std::panic::catch_unwind(|| extract_tpl_cooked_value(&elem));
  assert!(result.is_err());
}

/// Borrowed string extraction should panic on invalid UTF-8 payloads.
#[test]
fn convert_atom_to_str_ref_invalid_utf8_panics() {
  let atom = invalid_wtf8_atom();
  let result = std::panic::catch_unwind(|| convert_atom_to_str_ref(&atom));
  assert!(result.is_err());
}
