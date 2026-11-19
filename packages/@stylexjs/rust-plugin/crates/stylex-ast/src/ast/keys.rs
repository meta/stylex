//! Reading the static name of an object property or member key.

use stylex_constants::constants::messages::{
  ILLEGAL_PROP_VALUE, KEY_HAS_NO_NAME, SPREAD_NOT_SUPPORTED,
};
use stylex_macros::stylex_panic;
use stylex_utils::number::to_js_string;
use swc_core::{
  atoms::Atom,
  ecma::ast::{Expr, KeyValueProp, Lit, MemberProp, ObjectLit, Prop, PropName, PropOrSpread},
};

use super::convertors::{convert_str_lit_to_atom, convert_tpl_to_string_lit, normalize_expr};

pub fn namespace_name_from_prop_key(key: &PropName) -> Option<Atom> {
  match key {
    PropName::Ident(ident) => Some(ident.sym.clone()),
    PropName::Str(strng) => Some(convert_str_lit_to_atom(strng)),
    // Spelled the way JavaScript spells the number (`1e21` names `1e+21`).
    PropName::Num(num) => Some(Atom::from(to_js_string(num.value))),
    PropName::BigInt(big_int) => Some(Atom::from(big_int.value.to_string())),
    PropName::Computed(computed) => namespace_name_from_expr(computed.expr.as_ref()),
  }
}

/// The authored name of a key, or `None` (including for invalid UTF-8; never panics).
/// [`namespace_name_from_prop_key`] refuses invalid UTF-8 instead.
pub fn try_namespace_name_from_prop_key(key: &PropName) -> Option<Atom> {
  match key {
    PropName::Ident(ident) => Some(ident.sym.clone()),
    PropName::Str(strng) => strng.value.as_atom().cloned(),
    PropName::Num(num) => Some(Atom::from(to_js_string(num.value))),
    PropName::BigInt(big_int) => Some(Atom::from(big_int.value.to_string())),
    PropName::Computed(computed) => try_namespace_name_from_expr(computed.expr.as_ref()),
  }
}

fn try_namespace_name_from_lit(lit: &Lit) -> Option<Atom> {
  match lit {
    Lit::Str(strng) => strng.value.as_atom().cloned(),
    Lit::Num(num) => Some(Atom::from(to_js_string(num.value))),
    Lit::BigInt(big_int) => Some(Atom::from(big_int.value.to_string())),
    _ => None,
  }
}

fn try_namespace_name_from_expr(expr: &Expr) -> Option<Atom> {
  // Read through parentheses, matching `convert_member_prop_to_string`.
  match normalize_expr(expr) {
    Expr::Lit(lit) => try_namespace_name_from_lit(lit),
    // A template with one quasi and no interpolation names that quasi's text.
    Expr::Tpl(tpl) if tpl.exprs.is_empty() && tpl.quasis.len() == 1 => tpl.quasis[0]
      .cooked
      .as_ref()
      .and_then(|cooked| cooked.as_atom().cloned()),
    _ => None,
  }
}

pub fn namespace_name_from_member_prop(prop: &MemberProp) -> Option<Atom> {
  match prop {
    MemberProp::Ident(ident) => Some(ident.sym.clone()),
    MemberProp::Computed(computed) => namespace_name_from_expr(computed.expr.as_ref()),
    MemberProp::PrivateName(_) => None,
  }
}

fn namespace_name_from_lit(lit: &Lit) -> Option<Atom> {
  match lit {
    Lit::Str(strng) => Some(convert_str_lit_to_atom(strng)),
    Lit::Num(num) => Some(Atom::from(to_js_string(num.value))),
    Lit::BigInt(big_int) => Some(Atom::from(big_int.value.to_string())),
    _ => None,
  }
}

fn namespace_name_from_expr(expr: &Expr) -> Option<Atom> {
  // Read through parentheses, as in [`try_namespace_name_from_expr`].
  match normalize_expr(expr) {
    Expr::Lit(lit) => namespace_name_from_lit(lit),
    Expr::Tpl(tpl) => convert_tpl_to_string_lit(tpl)
      .as_ref()
      .and_then(namespace_name_from_lit),
    _ => None,
  }
}

/// The literal keys of an object literal, in source order, duplicates included.
pub fn collect_object_lit_keys(object: &ObjectLit) -> impl Iterator<Item = Atom> + '_ {
  object.props.iter().filter_map(|prop| {
    prop_as_key_value(prop).and_then(|key_value| namespace_name_from_prop_key(&key_value.key))
  })
}

/// A property's name and key-value; panics on any other shape (only reachable by hand).
pub fn named_key_value(prop: &PropOrSpread) -> (Atom, &KeyValueProp) {
  let key_value = match prop {
    PropOrSpread::Spread(_) => stylex_panic!("{}", SPREAD_NOT_SUPPORTED),
    PropOrSpread::Prop(prop) => match prop.as_ref() {
      Prop::KeyValue(key_value) => key_value,
      _ => stylex_panic!("{}", ILLEGAL_PROP_VALUE),
    },
  };

  match namespace_name_from_prop_key(&key_value.key) {
    Some(name) => (name, key_value),
    None => stylex_panic!("{}", KEY_HAS_NO_NAME),
  }
}

/// Returns the key-value of a `Prop::KeyValue`; `None` for any other shape.
pub fn prop_as_key_value(prop: &PropOrSpread) -> Option<&KeyValueProp> {
  match prop {
    PropOrSpread::Prop(p) => match p.as_ref() {
      Prop::KeyValue(kv) => Some(kv),
      _ => None,
    },
    _ => None,
  }
}
