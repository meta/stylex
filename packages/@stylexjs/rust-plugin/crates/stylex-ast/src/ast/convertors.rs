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

/// Reads the static key of a member property as a string:
/// - non-computed identifier → the identifier name
/// - computed string literal → the string value (`None` on invalid UTF-8)
/// - computed numeric literal → JS `String(Number)` rendering
///
/// Returns `None` for private names and non-literal computed keys, so callers
/// can treat a dynamic key as "not statically resolvable".
pub fn convert_member_prop_to_string(prop: &MemberProp) -> Option<String> {
  match prop {
    MemberProp::Ident(ident) => Some(ident.sym.to_string()),
    MemberProp::Computed(computed) => {
      convert_static_member_key_expr_to_string(normalize_expr(computed.expr.as_ref()))
    },
    MemberProp::PrivateName(_) => None,
  }
}

/// Unwraps parenthesized expressions, returning a reference to the innermost
/// non-paren expression. Spans are preserved. Use [`normalize_expr_mut`] when
/// the caller needs to mutate the unwrapped node.
///
/// Unwrapped in a loop rather than by recursing, because a caller inside the
/// evaluator's guard asks this before it descends and so has no nesting budget
/// to spend. A loop needs none.
pub fn normalize_expr(mut expr: &Expr) -> &Expr {
  while let Expr::Paren(paren) = expr {
    expr = paren.expr.as_ref();
  }
  expr
}

/// The call a declarator is initialised by, read through any parentheses
/// around it.
///
/// One rule rather than one spelling per site. A parenthesis is not a different
/// initializer, so `const fade = (stylex.keyframes({...}))` names the same call
/// `const fade = stylex.keyframes({...})` does -- and a reader that missed one
/// of them left the call untransformed and let the name reach the runtime. The
/// predicate that recognises such a declarator, the validator that checks it
/// and the transform that rewrites it each asked this question, so they ask it
/// in one place and cannot come to answer it differently.
///
/// `None` is a declarator with no initializer, or one initialised by anything
/// that is not a call.
pub fn init_call(var_declarator: &VarDeclarator) -> Option<&CallExpr> {
  var_declarator
    .init
    .as_deref()
    .map(normalize_expr)
    .and_then(Expr::as_call)
}

/// Mutable counterpart to [`normalize_expr`]: unwraps parenthesized
/// expressions, returning a mutable reference to the innermost non-paren
/// expression. Spans are preserved, so callers that depend on position
/// information (span containment, or span-insensitive comparison via
/// `eq_ignore_span`) keep working on the returned node. This is the
/// read/mutate counterpart to [`crate::ast::factories::wrap_in_paren`].
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

/// The numeric literal one number is written as, spelled the way JavaScript
/// spells it.
///
/// [`create_number_expr`] leaves the spelling to the emitter, which writes
/// most numbers the same way. Three it does not: it writes `-0` where the
/// language writes `0`, and it has no numeral at all for `NaN` or for an
/// infinity, so it invents `0 / 0` and `1 / 0`. Where the text is read by a
/// person as well as by the runtime -- an inline style, an injected constant
/// -- the spelling is given here rather than invented there.
///
/// The text is carried as the literal's own raw form. Nothing reads it back,
/// and every reader of the node takes the value beside it, so a spelling such
/// as `-Infinity` that is not a numeral is still the right text.
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

/// Whether an identifier is the language's `undefined`.
///
/// One predicate, because `undefined` is a *value* here rather than a name that
/// failed to resolve -- `js_undefined()` is what a key an object does not carry,
/// an index past the end of an array and a member read off a fold all answer, so
/// several steps have to recognise it on the way back out. Four private copies
/// of this test is how they could come to disagree about which of them is
/// looking at a value and which at an unresolved reference.
///
/// A binding named `undefined` that shadows the global never arrives: the
/// evaluator refuses a shadowed `undefined` ahead of any of these readers.
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

/// The key-value pair a property stands for, copied only where the copy is the
/// point.
///
/// A shorthand name is the one spelling that has to be rewritten before it can
/// be read as a pair, and it holds nothing but the name. Every other property
/// is already a pair and is read where it lies. A reader that copies first
/// copies the whole value subtree of the property, which for a namespace of a
/// `create` call is the entire style object.
pub fn expanded_shorthand_prop(prop: &Prop) -> Cow<'_, Prop> {
  match prop.as_shorthand() {
    Some(ident) => Cow::Owned(Prop::from(KeyValueProp {
      key: convert_string_to_prop_name(ident.sym.as_ref()),
      value: Box::new(Expr::Ident(ident.clone())),
    })),
    None => Cow::Borrowed(prop),
  }
}

/// Rewrites a shorthand name in place into the key-value pair it stands for.
///
/// For a caller that owns the property and reads it by mutable borrow. One
/// that only reads it asks [`expanded_shorthand_prop`] and pays no copy.
pub fn expand_shorthand_prop(prop: &mut Box<Prop>) {
  if let Cow::Owned(expanded) = expanded_shorthand_prop(prop) {
    **prop = expanded;
  }
}

/// The length of a string literal's value as JavaScript reports it: its count
/// of UTF-16 code units.
///
/// Reads the atom rather than a `String`, because a JavaScript string literal
/// can hold an unpaired surrogate and no `String` can. `"\uD83D"` is a legal
/// string of length 1, so `convert_atom_to_string` would abort the build on an
/// input whose length is the one thing that needs no valid scalar to answer —
/// and it would abort from inside an evaluation that is allowed to fail.
///
/// The valid-UTF-8 path goes through `utf16_length` rather than counting again
/// here, so a string and its literal cannot end up measured by two conventions
/// that drifted apart.
pub fn atom_utf16_length(atom: &Wtf8Atom) -> usize {
  match atom.as_str() {
    Some(value) => utf16_length(value),
    None => atom.to_ill_formed_utf16().count(),
  }
}

/// The character at a UTF-16 index of `atom`, as a value this compiler carries.
///
/// The language indexes a string by code unit, so this counts the units rather
/// than the Rust characters: the two agree inside the Basic Multilingual Plane
/// and part company on an astral character, which is two units and one
/// character.
///
/// `None` is an index past the end, which the caller reads as `undefined`.
///
/// A unit that is half of an astral character has no `char` of its own, and
/// answers the replacement character -- the same substitution the engine fold
/// makes for every string it carries back, and for the same reason: the
/// reference implementation's own output becomes that character once it is
/// written to a file, so the declaration text agrees and only the class name
/// parts. Reading the same character through `charAt` already answers this way,
/// and one read written two ways must not answer two things.
///
/// This is a decided exemption from `guidelines/stack/RUST.md`, which says a
/// substitution is safe only where the value it puts in cannot become part of a
/// folded answer. Here it can: the fixture
/// `a_string_index_that_lands_on_half_a_character.js` shows `content` reaching
/// the stylesheet with the replacement character in it. It is taken anyway for
/// the reason above -- the reference implementation writes the same character
/// to the same place -- and is named here so a reader finds the decision rather
/// than the departure.
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

/// The text of a string literal, borrowed rather than copied.
///
/// A literal already holds its text, so a reader that only compares or sorts
/// the text does not have to copy it. Refuses the same way the owned form does,
/// because it is the same reading. Text that is not valid UTF-8 holds a lone
/// surrogate, which Rust has no `str` for.
///
/// [`extract_str_lit_ref`] reads the same field and refuses with a different
/// sentence. The two are kept apart because each sentence is what its own
/// callers already print, and joining them would change a message an author
/// reads.
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

/// The authored name of a key, borrowed where the key already holds it.
///
/// Two of the five shapes hold their own name -- an identifier and a string --
/// so a reader that only compares or sorts keys borrows it. The other three
/// spell a name the key does not hold: a number as JavaScript spells it, a big
/// integer as its digits, and a computed key as the literal it folds to.
///
/// One rule for what a key is called.
/// [`convert_key_value_to_str`] is this answer, made owned, so a reader that
/// needs the text to outlive the key still asks the same question.
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

/// The authored name of a key, owned.
///
/// The name is answered as it is written, with no quotes around it.
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
      // The pair that is answered is copied, and nothing else is. Reading the
      // property where it lies used to cost a copy of the whole value subtree
      // of every property first, at every one of this reader's call sites.
      // A shorthand name still costs two small copies, because the pair built
      // for it is copied again to be answered, and both hold only the name.
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
/// Panics (via `stylex_panic!`) when the declarator has no initializer, e.g.
/// `let x;`. Callers that may encounter uninitialized declarators must guard on
/// `var_decl.init.is_some()` first.
pub fn get_expr_from_var_decl(var_decl: &VarDeclarator) -> &Expr {
  match &var_decl.init {
    Some(var_decl_init) => var_decl_init,
    None => stylex_panic!("Variable declaration must be initialized with an expression."),
  }
}
