//! The StyleX functions the compile-time engine is allowed to call.
//!
//! The [folded function map](../../../../../CONTEXT.md#folded-function-map) is
//! not JavaScript: its values are this compiler's own Rust functions, and most
//! of them answer by writing into the build — `keyframes` injects a rule and
//! hands back the name it hashed. Carrying the map into the engine as a value
//! was measured and rejected for exactly that reason.
//!
//! One of them answers from its arguments and nothing else. `firstThatWorks`
//! reorders the fallbacks it was handed and folds the CSS variables among them
//! into one `var()` chain, touching no state on the way, so there is nothing to
//! stop the engine running it — and nothing an author could reach through it
//! that they could not reach by calling it directly. It travels as a function of
//! the engine's own, built over the same [shared
//! core](../../stylex_first_that_works.rs) the evaluator's
//! own path calls, so the two cannot come to answer differently.
//!
//! What that buys is a callback: `a.map(x => firstThatWorks(x, 'serif'))` is
//! one JavaScript call per element, which is the engine's job and not this
//! compiler's. Every other StyleX function stays where it was — the fold hands
//! its call back, and the dispatch behind the fold calls it as it always has.

use boa_engine::{
  Context, JsResult, JsString, JsValue, native_function::NativeFunctionPointer,
  object::builtins::JsArray,
};
use swc_core::{
  atoms::Atom,
  ecma::ast::{Expr, MemberExpr, MemberProp},
};

use stylex_ast::ast::convertors::normalize_expr;
use stylex_constants::constants::api_names::STYLEX_FIRST_THAT_WORKS;

use crate::stylex_first_that_works::{
  Fallbacks, cut_to_css_variable_name, fold_fallback_chain, plan_fallbacks,
};
use stylex_state::state_manager::{ImportKind, StateManager};

/// One StyleX function the engine may call: how the module can name it, and what
/// the engine runs in the compiler's place.
struct Callable {
  /// The import this function is, which is how a bare name is recognised — the
  /// module's import record says which StyleX function a name is, where a
  /// function map entry only says that some Rust function stands behind it and
  /// every one of those looks alike from here.
  kind: ImportKind,
  /// The property a namespace exposes it under, which is how `stylex.…` is
  /// recognised.
  property: &'static str,
  call: NativeFunctionPointer,
}

/// Every StyleX function the engine may call.
///
/// A set rather than a table standing in for the language: what it holds is the
/// answer to one question — does this function answer from its arguments alone —
/// and that question has no source to read it off. Adding a name to it is a
/// claim about that function, and the [namespace
/// object](Reached::AsAProperty) is built from the whole of it so that what a
/// namespace name carries stays a function of the name.
const CALLABLE: [Callable; 1] = [Callable {
  kind: ImportKind::FirstThatWorks,
  property: STYLEX_FIRST_THAT_WORKS,
  call: first_that_works,
}];

/// A StyleX function the engine may call, and the one name the printed source
/// needs a value for to reach it.
pub(super) struct EngineCallable {
  /// The name that becomes a parameter of the printed arrow — the local name of
  /// an imported function, or the namespace it is read off.
  pub(super) name: Atom,
  /// Which of those two the name is, since the value behind it differs.
  pub(super) reached: Reached,
  /// The function the call itself names, which is what runs and what a refusal
  /// is reported under.
  called: &'static Callable,
}

/// How the source names the function.
pub(super) enum Reached {
  /// `firstThatWorks(…)` — the name is the function's own, and it carries the
  /// function.
  Directly,
  /// `stylex.firstThatWorks(…)` — the name is the namespace's, so it carries an
  /// object and the printed call reads exactly as it was written.
  ///
  /// The object holds every function of [`CALLABLE`] rather than the one this
  /// call named, so a fold naming two of them carries one object holding both:
  /// the transport keeps one value per name, and a per-call-site object would
  /// have let the second naming be dropped. Every other property of the real
  /// namespace is a function the engine may not call, and a call naming one is
  /// declined before anything is printed.
  AsAProperty,
}

impl EngineCallable {
  /// The function's own spelling, which is what a refusal names: the namespace
  /// in front of it is not the half an author has to change.
  pub(super) fn function_name(&self) -> &'static str {
    self.called.property
  }

  /// What the engine runs in the compiler's place, for a name that holds the
  /// function itself.
  pub(super) fn call(&self) -> NativeFunctionPointer {
    self.called.call
  }

  /// The whole callable surface a namespace name has to hold, which is every
  /// function of [`CALLABLE`] rather than the one this call named — see
  /// [`Reached::AsAProperty`].
  pub(super) fn namespace_properties() -> impl Iterator<Item = (&'static str, NativeFunctionPointer)>
  {
    CALLABLE
      .iter()
      .map(|callable| (callable.property, callable.call))
  }
}

/// The StyleX function `callee` names, or `None` where it names none.
pub(super) fn engine_callable(
  callee: &Expr,
  traversal_state: &StateManager,
) -> Option<EngineCallable> {
  // A parenthesis is not a different callee, at either level: `(stylex.types)`
  // and `(stylex).types` name what `stylex.types` names. Only names are read
  // here, so nothing is printed back and unwrapping costs no spelling.
  match normalize_expr(callee) {
    Expr::Ident(ident) => CALLABLE
      .iter()
      .find(|callable| traversal_state.any_stylex_api_import_contains(&[callable.kind], &ident.sym))
      .map(|called| EngineCallable {
        name: ident.sym.clone(),
        reached: Reached::Directly,
        called,
      }),
    Expr::Member(MemberExpr {
      obj,
      prop: MemberProp::Ident(prop),
      ..
    }) => {
      let namespace = normalize_expr(obj).as_ident()?;

      if !traversal_state.is_regular_stylex_import(&namespace.sym) {
        return None;
      }

      CALLABLE
        .iter()
        .find(|callable| prop.sym == callable.property)
        .map(|called| EngineCallable {
          name: namespace.sym.clone(),
          reached: Reached::AsAProperty,
          called,
        })
    },
    _ => None,
  }
}

/// `firstThatWorks` over the engine's own values.
///
/// The ordering is [`plan_fallbacks`]'s, so this and the evaluator's own path
/// cannot disagree about which argument falls back to which. What differs is
/// only how a value is read: an argument is a variable reference when it *is* a
/// string matching one, where the evaluator's path reads every argument as text
/// first. So a number or an array among the fallbacks keeps its own form and
/// reaches CSS the way the language writes it, which is what the reference
/// implementation does too.
///
/// Text is read out of the engine's UTF-16 strings, which can hold an unpaired
/// surrogate that no Rust string can. Such a unit becomes the replacement
/// character here, as it does everywhere a folded string crosses back.
fn first_that_works(_this: &JsValue, args: &[JsValue], engine: &mut Context) -> JsResult<JsValue> {
  // Read once per argument rather than per question. Reading a string out of the
  // engine costs nothing and cannot fail, so there is no reason to be lazy about
  // it the way the evaluator's own path has to be.
  let names = args
    .iter()
    .map(|value| {
      let mut text = value.as_string()?.to_std_string_lossy();

      cut_to_css_variable_name(&mut text).then_some(text)
    })
    .collect::<Vec<_>>();

  // The chain's text is read the same way whichever shape holds it.
  let folded = |chain: &[usize], engine: &mut Context| -> JsResult<JsValue> {
    let mut parts = Vec::with_capacity(chain.len());

    for &index in chain {
      parts.push(match &names[index] {
        Some(name) => name.clone(),
        // The value the chain bottoms out on, read as the language reads it —
        // which is what puts it inside `var(…, here)`.
        None => args[index].to_string(engine)?.to_std_string_lossy(),
      });
    }

    Ok(JsValue::from(JsString::from(fold_fallback_chain(parts))))
  };

  match plan_fallbacks(args.len(), |index| names[index].is_some()) {
    Fallbacks::Reversed => Ok(JsValue::from(JsArray::from_iter(
      args.iter().rev().cloned(),
      engine,
    ))),
    Fallbacks::Chain(chain) => folded(&chain, engine),
    Fallbacks::ChainAndRest(chain, rest) => {
      let chain = folded(&chain, engine)?;
      let values = std::iter::once(chain)
        .chain(rest.iter().map(|&index| args[index].clone()))
        .collect::<Vec<_>>();

      Ok(JsValue::from(JsArray::from_iter(values, engine)))
    },
  }
}

// What the call answers for a fallback that throws when it is read, which no
// transported value can be.
#[cfg(test)]
#[path = "tests/throwing_fallback_tests.rs"]
mod throwing_fallback_tests;
