//! A `defineVars` group as a value the engine can read members off.
//!
//! The group is not a table of members. Any name read off it answers a variable
//! derived from the group's own identity — the file and export that declare it —
//! which is why this compiler holds it as a reference rather than as a map, and
//! why it used to cross into the engine as nothing more than the string its own
//! `toString` answers. A string has none of the group's members, so every
//! expression that read one was handed back — and once the whole of
//! `Array.prototype` belonged to the fold there was nothing below to hand it
//! back to.
//!
//! What the language has for a value whose members are answered rather than
//! stored is a proxy, so that is what crosses. The identity travels as the two
//! plain values a member's name is derived from, and one Rust function derives
//! it — the same one [`ThemeRef::get`](
//! stylex_state::theme_ref::ThemeRef::get) calls, so the engine and
//! the evaluator cannot come to answer the same read differently.
//!
//! The traps are written in JavaScript rather than assembled here because the
//! `toString` a group needs is a closure over the group's hash, and the engine's
//! safe surface builds native functions from plain function pointers, which
//! capture nothing.

use boa_engine::{
  Context, JsError, JsNativeError, JsObject, JsResult, JsString, JsValue, NativeFunction, Source,
  js_string,
  object::{
    FunctionObjectBuilder,
    builtins::{JsArray, JsFunction},
  },
};
use rustc_hash::FxHashSet;

use stylex_constants::constants::common::VAR_GROUP_HASH_KEY;
use stylex_constants::constants::evaluation_errors::{
  engine_did_not_start, unfoldable_fold_result,
};

use swc_core::atoms::Atom;

use super::Decline;
use super::engine::read;
use stylex_state::theme_ref::{IS_PROXY_KEY, ThemeRef, var_group_member};

/// What the derivation throws when the traps did not hand it an identity.
///
/// Named rather than written where it is thrown, so the case that asserts it
/// reads the same words the throw does. An author never meets this sentence: it
/// says the traps and the derivation disagree, which is a fault of this
/// compiler and not of any source.
const READ_WITHOUT_AN_IDENTITY: &str = "A theme group was read without its own identity.";

/// How many values the traps hand the derivation.
///
/// The traps build that list from named locals and the derivation reads it back
/// as one slice pattern, so the two orders agree by the order each is written
/// in. The engine is told this number as the derivation's arity.
const IDENTITY_ARITY: usize = 3;

/// The traps a `defineVars` group answers reads through.
///
/// Takes the derivation as its one argument and hands back the builder the fold
/// calls per group, so the derivation is one function object per engine rather
/// than one per group.
///
/// The three keys answered ahead of the derivation are the stand-in's own rather
/// than any member's: one says what the value is, one is its own text, one is how
/// the language asks a value for that text. A key that is not a string is not a
/// member name at all. Everything else is a member, which is what makes a group a
/// proxy rather than an object — the member nobody wrote down answers exactly as
/// the one somebody did.
///
/// A group whose members are themselves groups is one *token* per dotted path
/// rather than a read of a read: `colors.brand.primary` names `brand.primary`.
/// Which paths those are is a question about the source and not about any value,
/// so the guard answers it and hands the prefixes down — a key it named stands in
/// for the path so far, and every other key answers the variable its path names.
/// A prefix nobody wrote is a member like any other, which is why the same read
/// off a *value* the expression produced still answers a string.
///
/// Only reads are trapped. Every static that writes to an object is refused by
/// name before it reaches a fold, and no assignment is admitted at all, so a
/// write trap would be a rule nothing can reach.
///
/// Written out rather than assembled from pieces, because it is JavaScript and
/// reads as JavaScript. The two keys it compares against are the compiler's own
/// constants, so a rename reaches this source rather than passing it by.
pub(super) fn var_group_traps() -> String {
  format!(
    r#"(member) => (baseId, prefix, paths) => {{
      const identity = [baseId, prefix];
      const nested = new Set(paths);

      const standIn = (path) => {{
        const text = path === "" ? member(...identity, "{group}") : member(...identity, path);

        return new Proxy({{}}, {{
          get(_, key) {{
            if (typeof key !== "string") return undefined;
            if (key === "{proxy}") return true;
            if (key === "toString") return () => text;
            if (key === "{group}") return text;

            const full = path === "" ? key : path + "." + key;

            return nested.has(full) ? standIn(full) : member(...identity, full);
          }}
        }});
      }};

      return standIn("");
    }}"#,
    group = VAR_GROUP_HASH_KEY,
    proxy = IS_PROXY_KEY,
  )
}

/// An engine that could not be given what a `defineVars` group needs, in the
/// words the engine's own construction refuses with — it is the same step, and
/// an author reads one sentence for it either way.
fn unbuilt(reason: &str) -> Decline {
  Decline::rule(engine_did_not_start(reason))
}

/// The builder one engine answers every `defineVars` group with, evaluated once
/// when that engine is built.
///
/// Evaluated rather than assembled, and kept rather than re-evaluated: a group
/// crossing is a parse of the traps above otherwise, paid per group per fold.
///
/// Four steps, and each has a refusal of its own: the text has to parse, to a
/// function, which is called once, and has to answer a function. None of the
/// four can fire for [`var_group_traps`], which is the one source shipped — so
/// the source is a parameter, and a case hands in a source that fails the step
/// it is about. The refusals stay because the shipped source is assembled from
/// two of the compiler's constants, and a rename that breaks it is declined
/// here rather than folded past.
///
/// Answers a refusal rather than asserting, for the reason the engine's own
/// construction does — this runs inside an evaluation whose whole contract is
/// that it may fail.
pub(super) fn compile_traps(source: &str, context: &mut Context) -> Result<JsFunction, Decline> {
  let refused = |error: JsError| unbuilt(&error.to_string());

  let traps = context
    .eval(Source::from_bytes(source.as_bytes()))
    .map_err(refused)?;

  let Some(traps) = traps.as_callable() else {
    return Err(unbuilt(
      "the theme group traps did not compile to a function",
    ));
  };

  let derive = FunctionObjectBuilder::new(context.realm(), NativeFunction::from_fn_ptr(derive))
    .name(js_string!("member"))
    .length(IDENTITY_ARITY)
    .build();

  let built = traps
    .call(&JsValue::undefined(), &[derive.into()], context)
    .map_err(refused)?;

  match built.as_object().and_then(JsFunction::from_object) {
    Some(builder) => Ok(builder),
    None => Err(unbuilt("the theme group traps did not answer a builder")),
  }
}

/// `theme` as the value the printed expression reads members off.
///
/// The identity crosses as the two values a member's name is derived from
/// rather than as the reference itself: nothing of this compiler's own can live
/// inside the engine, and these two are what the derivation needs. `prefixes`
/// is the third thing it needs and the one the values cannot say — see
/// [`var_group_traps`].
pub(super) fn var_group(
  builder: &JsFunction,
  theme: &ThemeRef,
  prefixes: Option<&FxHashSet<Atom>>,
  context: &mut Context,
) -> JsResult<JsValue> {
  let paths = prefixes
    .into_iter()
    .flatten()
    .map(|prefix| JsValue::from(JsString::from(prefix.as_str())));

  let paths = JsArray::from_iter(paths, context);

  builder.call(
    &JsValue::undefined(),
    &[
      JsString::from(theme.base_id()).into(),
      JsString::from(theme.class_name_prefix()).into(),
      paths.into(),
    ],
    context,
  )
}

/// Whether `value` is a group rather than something derived from one.
///
/// Asked of the answer a fold produced, in the two places a group standing where
/// an ordinary value was expected has to be answered for: alone at the top of an
/// answer, where the whole call is handed back to the dispatch that holds the
/// reference, and inside one, where the group converts to its own text.
///
/// Read off the value rather than predicted from the call, because what a fold
/// answers is a property of the whole chain and not of the method that ends it.
/// The key it reads is the one every reader of a group in this compiler asks.
pub(super) fn is_a_var_group(
  value: &JsValue,
  method: &Atom,
  context: &mut Context,
) -> Result<bool, Decline> {
  let Some(object) = value.as_object() else {
    return Ok(false);
  };

  let marked = read(method, || object.get(JsString::from(IS_PROXY_KEY), context))?;

  Ok(marked.to_boolean())
}

/// The text a group answers for itself, read off the group rather than derived a
/// second time here — the variable-group hash at the top of one, and the variable
/// a dotted path names below it.
///
/// What a group converts back to when it comes out of the engine inside an
/// answer. There is no expression this compiler can write for the group itself,
/// and its own `toString` is what the language would have used the moment
/// anything joined or printed the array holding it, so this is that answer taken
/// one step earlier.
pub(super) fn var_group_text(
  object: &JsObject,
  method: &Atom,
  context: &mut Context,
) -> Result<JsString, Decline> {
  let hash = read(method, || {
    object.get(JsString::from(VAR_GROUP_HASH_KEY), context)
  })?;

  match hash.as_string() {
    Some(hash) => Ok(hash),
    // Unreachable while the traps above answer that key, and answered rather
    // than asserted for the reason every refusal in this module is. Reported as
    // an answer the bridge cannot read back, which is where it happens — the
    // engine started, and a fold is running.
    None => Err(Decline::rule(unfoldable_fold_result(
      "theme group with no text of its own",
    ))),
  }
}

/// The CSS one member read answers, derived by the compiler's own naming.
///
/// Reached only from the traps above, which pass the identity they were built
/// with — so an argument of the wrong shape is a broken invariant rather than
/// anything an author can write. It throws all the same: this runs inside an
/// evaluation whose whole contract is that it may fail, where an assertion would
/// abort a build a refusal would only decline.
///
/// The list is read as one pattern rather than one index at a time, so the whole
/// read asks the length once and has one missing case to answer rather than one
/// per value. A longer list is the traps of a later version and is read as this
/// one.
fn derive(_: &JsValue, arguments: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
  let broken = || JsError::from_native(JsNativeError::typ().with_message(READ_WITHOUT_AN_IDENTITY));

  let [base_id, class_name_prefix, key, ..] = arguments else {
    return Err(broken());
  };

  let text = |value: &JsValue| match value.as_string() {
    Some(text) => Ok(text.to_std_string_escaped()),
    None => Err(broken()),
  };

  let named = var_group_member(&text(base_id)?, &text(class_name_prefix)?, &text(key)?);

  Ok(JsString::from(named).into())
}

#[cfg(test)]
#[path = "tests/var_group_tests.rs"]
mod var_group_tests;
