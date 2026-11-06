use std::borrow::Cow;

use anyhow::anyhow;
use stylex_macros::{stylex_panic, stylex_unimplemented};
use stylex_utils::{number::to_js_string, string::utf16_length, swc::get_expr_node_kind};
use swc_core::{
  atoms::{Atom, Wtf8Atom},
  common::DUMMY_SP,
  ecma::{
    ast::{
      BigInt, Bool, CallExpr, Expr, Ident, KeyValueProp, Lit, MemberProp, Number, ObjectLit, Prop,
      PropName, PropOrSpread, Str, Tpl, TplElement, VarDeclarator,
    },
    parser::Context,
    utils::{quote_ident, quote_str},
  },
};

use stylex_constants::constants::messages::{
  ILLEGAL_PROP_VALUE, INVALID_UTF8, SPREAD_NOT_SUPPORTED,
};

use super::factories::{
  create_big_int_lit, create_boolean_lit, create_ident, create_null_lit, create_number_lit,
  create_string_lit,
};

/// Reads a member property's static key: ident name, string value, or JS `String(Number)`.
/// `None` for private names, non-literal computed keys and invalid UTF-8.
pub fn convert_member_prop_to_string(prop: &MemberProp) -> Option<String> {
  match prop {
    MemberProp::Ident(ident) => Some(ident.sym.to_string()),
    MemberProp::Computed(computed) => {
      convert_static_member_key_expr_to_string(normalize_expr(computed.expr.as_ref()))
    },
    MemberProp::PrivateName(_) => None,
  }
}

/// Unwraps parentheses (iteratively, spans preserved); see [`normalize_expr_mut`].
pub fn normalize_expr(mut expr: &Expr) -> &Expr {
  while let Expr::Paren(paren) = expr {
    expr = paren.expr.as_ref();
  }
  expr
}

/// The call a declarator is initialized by, read through parentheses; `None` otherwise.
pub fn init_call(var_declarator: &VarDeclarator) -> Option<&CallExpr> {
  var_declarator
    .init
    .as_deref()
    .map(normalize_expr)
    .and_then(Expr::as_call)
}

/// Mutable counterpart to [`normalize_expr`]; spans are preserved.
pub fn normalize_expr_mut(mut expr: &mut Expr) -> &mut Expr {
  while let Expr::Paren(paren) = expr {
    expr = paren.expr.as_mut();
  }
  expr
}

fn convert_static_member_key_expr_to_string(expr: &Expr) -> Option<String> {
  match expr {
    Expr::Lit(Lit::Str(s)) => s.value.as_str().map(|value| value.to_string()),
    Expr::Lit(Lit::Num(n)) => Some(to_js_string(n.value)),
    Expr::Lit(Lit::BigInt(big_int)) => Some(big_int.value.to_string()),
    Expr::Tpl(tpl) => convert_tpl_to_string_lit(tpl).and_then(|lit| convert_lit_to_string(&lit)),
    _ => None,
  }
}

pub fn convert_lit_to_number(lit_num: &Lit) -> Result<f64, anyhow::Error> {
  match lit_num {
    Lit::Bool(Bool { value, .. }) => Ok(if *value { 1.0 } else { 0.0 }),
    Lit::Num(num) => Ok(num.value),
    Lit::Str(strng) => {
      let string_value = convert_atom_to_string(&strng.value);
      match string_value.parse::<f64>() {
        Ok(num) => Ok(num),
        Err(_) => Err(anyhow!("Value is not a number: {}", string_value)),
      }
    },
    _ => Err(anyhow!(
      "Value is not a number: {}",
      get_expr_node_kind(&Expr::from(lit_num.clone()))
    )),
  }
}

pub fn convert_tpl_to_string_lit(tpl: &Tpl) -> Option<Lit> {
  if !tpl.exprs.is_empty() || tpl.quasis.len() != 1 {
    return None;
  }

  let quasi = &tpl.quasis[0];
  let value = match quasi.cooked.as_ref() {
    Some(cooked) => match cooked.as_str() {
      Some(value) => value,
      None => stylex_panic!("Failed to extract a string value from the expression."),
    },
    None => stylex_panic!("Failed to extract cooked value from template literal element."),
  };

  Some(create_string_lit(value))
}

pub fn convert_simple_tpl_to_str_expr(expr: Expr) -> Expr {
  match expr {
    Expr::Tpl(ref tpl) => convert_tpl_to_string_lit(tpl).map_or(expr, Expr::Lit),
    _ => expr,
  }
}

pub fn convert_concat_to_tpl_expr(expr: Expr) -> Expr {
  match expr {
    Expr::Call(ref call_expr) => concat_call_to_template_literal(call_expr).unwrap_or(expr),
    _ => expr,
  }
}

pub(crate) fn concat_call_to_template_literal(call_expr: &CallExpr) -> Option<Expr> {
  use swc_core::common::DUMMY_SP;

  let callee = call_expr.callee.as_expr()?;
  let member_expr = callee.as_member()?;
  let prop_ident = member_expr.prop.as_ident()?;

  if prop_ident.sym.as_ref() != "concat" {
    return None;
  }

  let object_lit = member_expr.obj.as_lit()?;
  let base_string = {
    let base_string = extract_str_lit_ref(object_lit)?;
    base_string.to_string()
  };

  let mut exprs = Vec::new();
  let mut quasis = Vec::new();

  quasis.push(TplElement {
    span: DUMMY_SP,
    tail: false,
    cooked: Some(base_string.clone().into()),
    raw: base_string.into(),
  });

  for (i, arg) in call_expr.args.iter().enumerate() {
    if arg.spread.is_some() {
      continue;
    }

    exprs.push(arg.expr.clone());

    let is_last = i == call_expr.args.len() - 1;
    quasis.push(TplElement {
      span: DUMMY_SP,
      tail: is_last,
      cooked: Some("".into()),
      raw: "".into(),
    });
  }

  Some(Expr::Tpl(Tpl {
    span: DUMMY_SP,
    exprs,
    quasis,
  }))
}

pub fn create_number_expr(value: f64) -> Expr {
  Expr::from(create_number_lit(value))
}

/// A numeric literal spelled the way JavaScript spells it (`0` for `-0`, `NaN`, `Infinity`),
/// unlike [`create_number_expr`], which leaves the emitter to invent `0 / 0` and `1 / 0`.
pub fn create_js_number_expr(value: f64) -> Expr {
  Expr::Lit(Lit::Num(Number {
    span: DUMMY_SP,
    value,
    raw: Some(to_js_string(value).into()),
  }))
}

pub fn create_big_int_expr(value: BigInt) -> Expr {
  Expr::from(create_big_int_lit(value))
}

pub fn create_string_expr(value: &str) -> Expr {
  Expr::Lit(create_string_lit(value))
}

pub fn create_bool_expr(value: bool) -> Expr {
  Expr::Lit(create_boolean_lit(value))
}

/// Whether an identifier is the language's `undefined` (shadowing is refused upstream).
pub fn is_js_undefined(ident: &Ident) -> bool {
  ident.sym.as_ref() == "undefined"
}

pub fn create_ident_expr(value: &str) -> Expr {
  Expr::Ident(create_ident(value))
}

pub fn create_null_expr() -> Expr {
  Expr::Lit(create_null_lit())
}

fn should_wrap_prop_name_key_with_quotes(key: &str) -> bool {
  // Reserved words are lowercase ASCII, so any other ASCII character skips the lookups.
  if let Some((&first, rest)) = key.as_bytes().split_first()
    && first.is_ascii()
    && Ident::is_valid_ascii_start(first)
    && rest
      .iter()
      .all(|byte| byte.is_ascii() && Ident::is_valid_ascii_continue(*byte))
    && key.bytes().any(|byte| !byte.is_ascii_lowercase())
  {
    return false;
  }

  if Ident::verify_symbol(key).is_ok() {
    return false;
  }

  !Context::default().is_reserved_word(&key.into())
}

pub fn convert_string_to_prop_name(value: &str) -> PropName {
  if should_wrap_prop_name_key_with_quotes(value) {
    PropName::Str(quote_str!(value))
  } else {
    PropName::Ident(quote_ident!(value))
  }
}

/// The key-value pair a property stands for, copying only a shorthand name.
pub fn expanded_shorthand_prop(prop: &Prop) -> Cow<'_, Prop> {
  match prop.as_shorthand() {
    Some(ident) => Cow::Owned(Prop::from(KeyValueProp {
      key: convert_string_to_prop_name(ident.sym.as_ref()),
      value: Box::new(Expr::Ident(ident.clone())),
    })),
    None => Cow::Borrowed(prop),
  }
}

/// Rewrites a shorthand name in place into its key-value pair.
pub fn expand_shorthand_prop(prop: &mut Box<Prop>) {
  if let Cow::Owned(expanded) = expanded_shorthand_prop(prop) {
    **prop = expanded;
  }
}

/// A string literal's length in UTF-16 code units; handles unpaired surrogates without panicking.
pub fn atom_utf16_length(atom: &Wtf8Atom) -> usize {
  match atom.as_str() {
    Some(value) => utf16_length(value),
    None => atom.to_ill_formed_utf16().count(),
  }
}

/// The character at a UTF-16 index of `atom`; `None` past the end.
/// Half of an astral character answers U+FFFD, matching the engine fold and `charAt` (a decided
/// exemption from `guidelines/stack/RUST.md`).
pub fn atom_utf16_char_at(atom: &Wtf8Atom, index: usize) -> Option<char> {
  let unit = match atom.as_str() {
    Some(text) => text.encode_utf16().nth(index),
    None => atom.to_ill_formed_utf16().nth(index),
  }?;

  Some(char::from_u32(u32::from(unit)).unwrap_or(char::REPLACEMENT_CHARACTER))
}

pub fn convert_atom_to_string(atom: &Wtf8Atom) -> String {
  match atom.as_str() {
    Some(value) => value.to_string(),
    None => stylex_panic!("{}", INVALID_UTF8),
  }
}

pub fn convert_wtf8_to_atom(atom: &Wtf8Atom) -> Atom {
  match atom.as_atom() {
    Some(value) => value.clone(),
    None => stylex_panic!("{}", INVALID_UTF8),
  }
}

pub fn convert_str_lit_to_string(str_lit: &Str) -> String {
  convert_str_lit_to_str_ref(str_lit).to_string()
}

/// A string literal's text, borrowed; refuses lone surrogates like the owned form.
/// Kept apart from [`extract_str_lit_ref`] because each has a different user-facing message.
pub fn convert_str_lit_to_str_ref(str_lit: &Str) -> &str {
  match str_lit.value.as_str() {
    Some(value) => value,
    None => stylex_panic!("{}", INVALID_UTF8),
  }
}

pub fn convert_str_lit_to_atom(str_lit: &Str) -> Atom {
  match str_lit.value.as_atom() {
    Some(value) => value.clone(),
    None => stylex_panic!("{}", INVALID_UTF8),
  }
}

pub fn extract_tpl_cooked_value(elem: &TplElement) -> &str {
  match elem.cooked.as_ref() {
    Some(cooked) => match cooked.as_str() {
      Some(value) => value,
      None => stylex_panic!("{}", INVALID_UTF8),
    },
    None => stylex_panic!(
      "Template literal element has no cooked value (contains an invalid escape sequence)."
    ),
  }
}

pub fn convert_atom_to_str_ref(atom: &swc_core::atoms::Wtf8Atom) -> &str {
  match atom.as_str() {
    Some(value) => value,
    None => stylex_panic!("Failed to convert SWC Atom to string (invalid WTF-8 encoding)."),
  }
}

pub fn convert_lit_to_string(value: &Lit) -> Option<String> {
  match value {
    Lit::Str(strng) => Some(convert_str_lit_to_string(strng)),
    Lit::Num(num) => Some(to_js_string(num.value)),
    Lit::BigInt(big_int) => Some(format!("{}", big_int.value)),
    _ => None,
  }
}

pub fn extract_str_lit_ref(lit: &Lit) -> Option<&str> {
  match lit {
    Lit::Str(strng) => Some(convert_atom_to_str_ref(&strng.value)),
    _ => None,
  }
}

/// A key's authored name, borrowed for identifiers and strings; numbers, big integers and
/// computed keys are spelled out. [`convert_key_value_to_str`] is the owned form.
pub fn key_value_name(key_value: &KeyValueProp) -> Cow<'_, str> {
  match &key_value.key {
    PropName::Ident(ident) => Cow::Borrowed(ident.sym.as_str()),
    PropName::Str(strng) => Cow::Borrowed(convert_str_lit_to_str_ref(strng)),
    PropName::Num(num) => Cow::Owned(to_js_string(num.value)),
    PropName::BigInt(big_int) => Cow::Owned(big_int.value.to_string()),
    PropName::Computed(computed) => Cow::Owned(match computed.expr.as_ref() {
      Expr::Lit(lit) => match convert_lit_to_string(lit) {
        Some(s) => s,
        None => stylex_panic!("Computed property key must be a string or number literal."),
      },
      Expr::Tpl(tpl) => {
        match convert_tpl_to_string_lit(tpl).and_then(|lit| convert_lit_to_string(&lit)) {
          Some(s) => s,
          None => stylex_unimplemented!("Computed key is not a literal"),
        }
      },
      _ => stylex_unimplemented!("Computed key is not a literal"),
    }),
  }
}

/// A key's authored name, owned and unquoted.
#[inline]
pub fn convert_key_value_to_str(key_value: &KeyValueProp) -> String {
  key_value_name(key_value).into_owned()
}

pub fn get_key_values_from_object(object: &ObjectLit) -> Vec<KeyValueProp> {
  object
    .props
    .iter()
    .map(|prop| match prop {
      PropOrSpread::Spread(_) => stylex_unimplemented!("{}", SPREAD_NOT_SUPPORTED),
      // Copies only the answered pair, not the whole value subtree.
      PropOrSpread::Prop(prop) => match expanded_shorthand_prop(prop).as_ref() {
        Prop::KeyValue(key_value) => key_value.clone(),
        _ => stylex_panic!("{}", ILLEGAL_PROP_VALUE),
      },
    })
    .collect()
}

/// Extracts the initializer expression from a variable declarator.
///
/// # Panics
/// Via `stylex_panic!` when the declarator has no initializer (e.g. `let x;`).
pub fn get_expr_from_var_decl(var_decl: &VarDeclarator) -> &Expr {
  match &var_decl.init {
    Some(var_decl_init) => var_decl_init,
    None => stylex_panic!("Variable declaration must be initialized with an expression."),
  }
}
