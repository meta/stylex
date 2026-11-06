//! The two checks the printed source runs through, and the rewrite that puts
//! them there.
//!
//! The [guard](super::guard) answers every read and every call it can name from
//! the syntax. What it cannot name is a key that is a *value* — `o[k]`, where
//! `k` is a parameter or a name — because the key is only a string once the
//! engine has run. The language's own index operator then performs that read
//! under no rule of this compiler, and `({})['__proto__']['constructor']` walks
//! from a plain object to `Function`.
//!
//! So the printed source does not use the index operator for such a read. It
//! calls a reader that coerces the key, compares it against the one table the
//! guard reads, and throws the guard's own sentence — the same rule, applied
//! where the name finally exists.
//!
//! A caller stands behind the reader. It refuses the language's `Function`, the
//! three constructors that inherit directly from it, and `eval`, whatever route
//! produced them. Two guards rather than one is the point: a reader alone is
//! one unknown gadget away from an escape, and the gadget is exactly the one
//! nobody has found yet. The reference implementation ships both for the same
//! reason.
//!
//! The caller stands in front of one calling shape: a call through a name the
//! printed source itself binds. That is not a shortcut, it is the whole set. A
//! value the engine produced is invoked in one of three ways. Through a member
//! — where the method name is spelled out and the guard already read it, and
//! `call`, `apply`, `bind` and `constructor` are refused by name. Through an
//! expression the guard does not admit as a callee at all. Or through a name —
//! and a name is either bound inside the printed source, which is the shape
//! the escape used, or free. A free name is a parameter of the printed arrow,
//! holding a value or a function this compiler carried, or one of the globals
//! the guard lets stand free, which is the applied-global allowlist. Neither
//! can be a function that compiles code.
//!
//! So a call through a free name keeps the language's own call, and
//! `String(x)` costs what it always cost. If the guard is ever taught to leave
//! a new kind of name free, that reasoning has to be read again.
//!
//! Both are **parameters of the printed arrow**, not globals. A global is a
//! name the printed source could point somewhere else. A parameter is neither
//! reachable nor shadowable: the guard refuses a binding that spells one of
//! these two names, so nothing between the arrow and the read can stand in
//! front of it.
//!
//! The native that computes `**` is the one global the printed source calls
//! by name. Nothing can change what it holds: the property cannot be written,
//! and the guard refuses every name of its spelling. ADR 0008 gives the reason
//! that it is not a parameter.
//!
//! `docs/adr/0010-the-printed-fold-reads-and-calls-through-a-check.md` carries
//! the argument.

use boa_engine::{
  Context, JsError, JsString, JsValue, Source,
  object::builtins::{JsArray, JsFunction},
};
use rustc_hash::FxHashSet;
use swc_core::{
  atoms::Atom,
  common::util::take::Take,
  ecma::{
    ast::{BindingIdent, CallExpr, Callee, Expr, ExprOrSpread, Lit, MemberExpr, MemberProp},
    visit::{Visit, VisitMut, VisitMutWith, VisitWith},
  },
};

use stylex_ast::ast::factories::create_ident_call_expr;
use stylex_constants::constants::evaluation_errors::{
  BLOCKED_FUNCTION_CALL, BLOCKED_PROPERTY_ACCESS, engine_did_not_start,
};

use super::Decline;
use super::math::EXPONENTIATE;
use super::{BLOCKED_PROPERTIES, ESCAPING_PROPERTIES};

/// The name the printed source reads a computed property through.
pub(super) const READER: &str = "__sxRead";

/// The name the printed source calls a resolved function through.
pub(super) const CALLER: &str = "__sxCall";

/// The two checks, in the order the printed arrow binds them, and the key each
/// one is read off the built source under.
///
/// One table, because three readers depend on this order and a fourth on the
/// names: the parameter list the fold prints, the arguments passed beside it,
/// the reader that takes each check off the built source, and the guard's
/// refusal of a binding that would shadow either. The keys are spelled once
/// more in [`BACKSTOP_SOURCE`], which is the source that answers under them;
/// a source that spelled them differently is refused when the checks are
/// taken off it rather than folded past.
///
/// A third check is one line here and a compile error where the pair is read
/// apart, which is what the fixed width below buys over a list that would have
/// taken it in silence.
const CHECKS: [(&str, &str); 2] = [(READER, "read"), (CALLER, "call")];

/// The names the printed arrow binds, in the order it binds them.
pub(super) fn reserved_names() -> impl Iterator<Item = &'static str> {
  CHECKS.into_iter().map(|(name, _)| name)
}

/// Whether `name` is one the printed source reserves: a check of its own, or
/// the global native that computes `**` (see [`math`](super::math)).
pub(super) fn is_a_reserved_name(name: &str) -> bool {
  name == EXPONENTIATE || reserved_names().any(|reserved| reserved == name)
}

/// The property names the reader refuses: the guard's two tables and nothing
/// besides.
///
/// Chained rather than written out, so a name added to either table reaches the
/// engine without a second list agreeing to it. `constructor` is in both and
/// arrives twice, which the `Set` the reader builds from this answers once.
fn refused_property_names() -> impl Iterator<Item = &'static str> {
  ESCAPING_PROPERTIES.into_iter().chain(BLOCKED_PROPERTIES)
}

/// The reader and the caller, written as the JavaScript the engine compiles
/// once when it is built.
///
/// Takes its table and its two sentences as arguments rather than carrying them
/// as literals, so the compiler's own constants stay the only place either is
/// written and a multi-line sentence needs no escaping to get here.
///
/// Everything the checks themselves depend on is read once, while no fold has
/// run, and held in the closure. Reading `Function` or `Object.getPrototypeOf`
/// at check time would ask the global object a question at a moment a fold is
/// running.
///
/// The reader coerces the key exactly as the index operator would and then uses
/// the coerced name, so a key with a `toString` of its own is asked for its text
/// once rather than twice. A symbol is left uncoerced — coercing one throws, and
/// no symbol spells a refused name.
///
/// The caller answers the callee rather than calling it, so the engine performs
/// the call itself and no argument list is rebuilt: `f(x)` is printed as
/// `__sxCall(f)(x)`, which evaluates `f`, then the check, then `x` — the order
/// the language reads `f(x)` in, and with the same `this`.
///
/// It refuses the reference implementation's own triple: the language's
/// `Function`, anything inheriting directly from it, and `eval`.
///
/// The three constructors the second of those is written for — async, generator
/// and async-generator — are also named one by one, because the engine here
/// does not put them where the specification does: `Object.getPrototypeOf` of
/// its `GeneratorFunction` answers `Function.prototype` rather than `Function`,
/// so the inheritance test alone lets one through. Naming them is exact and
/// costs one set lookup. The inheritance test stays beside it, since it is the
/// general rule and the three are the cases it is known to miss.
pub(super) const BACKSTOP_SOURCE: &str = r#"(names, propertyMessage, functionMessage) => {
  const refused = new Set(names);
  const text = String;
  const prototypeOf = Object.getPrototypeOf;
  const compiler = Function;

  const compilers = new Set([
    compiler,
    eval,
    prototypeOf(function* () {}).constructor,
    prototypeOf(async function () {}).constructor,
    prototypeOf(async function* () {}).constructor,
  ]);

  return {
    read: (object, key) => {
      const name = typeof key === "symbol" ? key : text(key);

      if (refused.has(name)) {
        throw new TypeError(propertyMessage);
      }

      return object[name];
    },
    call: callee => {
      if (
        compilers.has(callee) ||
        (typeof callee === "function" && prototypeOf(callee) === compiler)
      ) {
        throw new TypeError(functionMessage);
      }

      return callee;
    },
  };
}"#;

/// The two checks as the engine holds them, built once per engine.
///
/// Kept beside the engine for the reason the theme builder is: what building
/// them costs is a parse, and the source parsed is the same one every fold on
/// that engine reads through.
///
/// Held in one list rather than one field each, and filled in [`CHECKS`]
/// order, so the order they are passed in is the order they are printed in by
/// construction rather than by two lists agreeing.
pub(super) struct Backstops {
  checks: [JsFunction; CHECKS.len()],
}

impl Backstops {
  /// The checks as the arguments the printed arrow takes them under.
  ///
  /// An iterator rather than a list, because the caller builds one list out of
  /// these and the carried values together.
  pub(super) fn arguments(&self) -> impl Iterator<Item = JsValue> + '_ {
    self.checks.iter().cloned().map(JsValue::from)
  }
}

/// An engine that could not be given its checks, in the words the engine's own
/// construction refuses with — it is the same step, and an author reads one
/// sentence for it either way.
fn unbuilt(reason: &str) -> Decline {
  Decline::rule(engine_did_not_start(reason))
}

/// Compiles `source` into the two checks every fold on this engine prints.
///
/// Four steps and a refusal each: the text has to parse, to a function, which
/// is called once, and has to answer an object holding both checks. None of the
/// four can fire for [`BACKSTOP_SOURCE`], which is the one source shipped — so
/// the source is a parameter and a case hands in one that fails the step it is
/// about. The keys are read in [`CHECKS`] order, which is the order the
/// arguments are passed in.
///
/// Answers a refusal rather than asserting, for the reason every other step of
/// building an engine does: this runs inside an evaluation whose whole contract
/// is that it may fail.
pub(super) fn compile_backstops(source: &str, context: &mut Context) -> Result<Backstops, Decline> {
  let refused = |error: JsError| unbuilt(&error.to_string());

  let built = context
    .eval(Source::from_bytes(source.as_bytes()))
    .map_err(refused)?;

  let Some(built) = built.as_callable() else {
    return Err(unbuilt("the fold's checks did not compile to a function"));
  };

  let names = JsArray::from_iter(
    refused_property_names().map(|name| JsValue::from(JsString::from(name))),
    context,
  );

  let checks = built
    .call(
      &JsValue::undefined(),
      &[
        names.into(),
        JsString::from(BLOCKED_PROPERTY_ACCESS).into(),
        JsString::from(BLOCKED_FUNCTION_CALL).into(),
      ],
      context,
    )
    .map_err(refused)?;

  // Read apart rather than collected, so the width stays fixed: a third entry
  // in `CHECKS` stops compiling here, where a list would have taken it and
  // left the printed parameter without an argument.
  let [(_, reader), (_, caller)] = CHECKS;

  Ok(Backstops {
    checks: [
      one_check(&checks, JsString::from(reader), context)?,
      one_check(&checks, JsString::from(caller), context)?,
    ],
  })
}

/// One of the two functions the source answers with, read off the pair it hands
/// back.
///
/// Both ways of not finding one read the same sentence, because an author can
/// act on neither: either says the shipped source and this reader disagree.
fn one_check(
  checks: &JsValue,
  named: JsString,
  context: &mut Context,
) -> Result<JsFunction, Decline> {
  let missing = || unbuilt("the fold's checks did not answer a reader and a caller");

  let Some(pair) = checks.as_object() else {
    return Err(missing());
  };

  let check = pair
    .get(named, context)
    .map_err(|error| unbuilt(&error.to_string()))?;

  match check.as_object().and_then(JsFunction::from_object) {
    Some(check) => Ok(check),
    None => Err(missing()),
  }
}

/// Rewrites every read and every call a check has to stand in front of,
/// answering whether it changed anything.
///
/// `false` is what keeps the cheapest fold cheap: an expression with no
/// computed key and no call through a name it binds itself is printed exactly
/// as it was, and needs neither the arrow nor the two arguments.
///
/// Run over one printed tree at a time — the call, and each declaration
/// crossing as a parameter default — because what a tree binds is read from
/// that tree. A default is source that really runs and gets the same reading;
/// the arrow the transport wraps them in is not one of them, since its
/// parameters are the names this compiler carried rather than names the source
/// bound.
///
/// Reached only where the fold memo missed, so the walk is a cost per printed
/// shape rather than per fold.
pub(crate) fn checked(expr: &mut Expr) -> bool {
  let mut bound = Bound::default();

  expr.visit_with(&mut bound);

  let mut rewrite = Rewrite {
    bound: bound.names,
    used: false,
  };

  expr.visit_mut_with(&mut rewrite);

  rewrite.used
}

/// Every name the printed tree binds, wherever it binds it.
///
/// One set for the whole tree rather than a stack of scopes. It over-counts —
/// a name bound in one callback makes a call through the same spelling in
/// another callback go through the check as well — and over-counting costs one
/// call and answers the same value. What it buys is that nothing here has to
/// model a scope, which is the language's job and not this walk's.
#[derive(Default)]
struct Bound {
  names: FxHashSet<Atom>,
}

impl Visit for Bound {
  fn visit_binding_ident(&mut self, ident: &BindingIdent) {
    self.names.insert(ident.sym.clone());
  }
}

/// Whether the syntax of a computed key spells the name it reads.
///
/// A string is the name itself, which the guard already ruled on. A number
/// spells no refused name and is how an element of an array is reached, so both
/// keep the language's own index operator. Everything else is a key that exists
/// only once the engine has run.
fn spells_a_name(key: &Expr) -> bool {
  matches!(key, Expr::Lit(Lit::Str(_) | Lit::Num(_)))
}

/// The expression a run of parentheses wraps.
fn without_parentheses(expr: &Expr) -> &Expr {
  match expr {
    Expr::Paren(paren) => without_parentheses(&paren.expr),
    other => other,
  }
}

/// An expression as one argument of the check it is handed to.
fn argument(expr: Expr) -> ExprOrSpread {
  ExprOrSpread {
    spread: None,
    expr: Box::new(expr),
  }
}

/// The walk that puts the checks in.
///
/// Bottom-up, so a read of a read is rewritten innermost first and the call
/// this builds is never visited again — which is what keeps the checks
/// themselves from being routed through a check.
struct Rewrite {
  /// The names the printed tree binds — see [`Bound`].
  bound: FxHashSet<Atom>,
  used: bool,
}

impl Rewrite {
  /// Whether a callee is a name the printed source bound, which is the one
  /// calling shape that can reach a function the guard never named.
  fn binds_the_callee(&self, callee: &Expr) -> bool {
    match without_parentheses(callee) {
      Expr::Ident(name) => self.bound.contains(&name.sym),
      _ => false,
    }
  }
}

impl VisitMut for Rewrite {
  fn visit_mut_expr(&mut self, expr: &mut Expr) {
    expr.visit_mut_children_with(self);

    // `o[k]`, where the key is only a name once the engine has run.
    if let Expr::Member(MemberExpr {
      obj,
      prop: MemberProp::Computed(key),
      ..
    }) = expr
      && !spells_a_name(&key.expr)
    {
      self.used = true;

      *expr = Expr::Call(create_ident_call_expr(
        READER,
        vec![argument(*obj.take()), argument(*key.expr.take())],
      ));
    }
  }

  /// The callee alone, so the arguments stay where they were written and the
  /// engine performs the call.
  fn visit_mut_call_expr(&mut self, call: &mut CallExpr) {
    call.visit_mut_children_with(self);

    match &mut call.callee {
      Callee::Expr(callee) if self.binds_the_callee(callee) => {
        self.used = true;

        // Written into the box the callee already sits in, so the check
        // replaces the callee without a second allocation for it.
        let callee: &mut Expr = callee;
        let named = callee.take();

        *callee = Expr::Call(create_ident_call_expr(CALLER, vec![argument(named)]));
      },
      // Every other callee is one no check has to stand in front of: a method
      // spells its name, and `super()` and `import()` are not expressions the
      // guard admits at all.
      _ => {},
    }
  }
}

#[cfg(test)]
#[path = "tests/backstop_tests.rs"]
mod backstop_tests;
