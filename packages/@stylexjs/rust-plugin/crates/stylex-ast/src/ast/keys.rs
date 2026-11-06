//! Reading the authored name of an object property or a member.
//!
//! A key is written in several shapes -- an identifier, a string, a number, a
//! big integer, or a computed literal -- and every one of them names the same
//! property at run time. These readers answer with that name, and answer with
//! nothing where the shape carries no static name at all.

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
    // A number key names a string property, spelled the way JavaScript spells
    // the number: `1e21` names `1e+21`, not the digit run Rust prints.
    PropName::Num(num) => Some(Atom::from(to_js_string(num.value))),
    PropName::BigInt(big_int) => Some(Atom::from(big_int.value.to_string())),
    PropName::Computed(computed) => namespace_name_from_expr(computed.expr.as_ref()),
  }
}

/// The authored name of a key, or nothing where the shape carries no static
/// name -- unreadable text included.
///
/// [`namespace_name_from_prop_key`] refuses text that is not valid UTF-8,
/// because a namespace key that cannot be spelled is a mistake worth
/// reporting. A caller that only asks whether a key names one particular prop
/// has no such stake: every other key in the object is none of its business,
/// so an unreadable one is simply not the key it asked about. This reader
/// answers that question and never panics.
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
  // A parenthesis is not a different key, so `{ [('a')]: 1 }` names what
  // `{ ['a']: 1 }` names. `convert_member_prop_to_string` already reads a
  // computed key this way, and the two must not disagree.
  match normalize_expr(expr) {
    Expr::Lit(lit) => try_namespace_name_from_lit(lit),
    // A template with one quasi and no interpolation names the text of that
    // quasi. The shape is read here rather than through the convertor, which
    // refuses the two cases this reader answers `None` for.
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
  // Read through the parentheses, for the reason
  // [`try_namespace_name_from_expr`] gives.
  match normalize_expr(expr) {
    Expr::Lit(lit) => namespace_name_from_lit(lit),
    Expr::Tpl(tpl) => convert_tpl_to_string_lit(tpl)
      .as_ref()
      .and_then(namespace_name_from_lit),
    _ => None,
  }
}

/// The literal keys of an object literal, in source order, duplicates included
/// -- a key written twice is two properties, and a count over them reads it that
/// way.
pub fn collect_object_lit_keys(object: &ObjectLit) -> impl Iterator<Item = Atom> + '_ {
  object.props.iter().filter_map(|prop| {
    prop_as_key_value(prop).and_then(|key_value| namespace_name_from_prop_key(&key_value.key))
  })
}

/// The name a property is declared under, and the property itself.
///
/// For a reader that was promised named key-values -- a property of an object
/// the evaluator folded, or one this compiler wrote -- so it asks for the name
/// and the property together and gets both. Any other shape is refused here
/// rather than skipped: such a reader has no reading for a property it cannot
/// name, and every one of them used to invent an answer of its own.
///
/// The refusals are reachable from an object built by hand, which is what the
/// cases beside this function do, and not from an object a producer of this
/// compiler wrote.
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

/// Returns `Some(kv)` only for `PropOrSpread::Prop(Box<Prop::KeyValue>)`
/// shapes; any other variant (spread, method, getter, setter, shorthand, …)
/// yields `None`. Callers typically use this to skip props they can't handle in
/// a single pass.
pub fn prop_as_key_value(prop: &PropOrSpread) -> Option<&KeyValueProp> {
  match prop {
    PropOrSpread::Prop(p) => match p.as_ref() {
      Prop::KeyValue(kv) => Some(kv),
      _ => None,
    },
    _ => None,
  }
}
