//! Folds a self-contained method call by evaluating it, rather than by
//! matching its name against a table of methods.
//!
//! A table is finite by construction, so the method it does not list is the
//! next bug report. Handing the call to a JavaScript engine instead covers
//! `String.prototype`, `Array.prototype` and `Object.prototype` at once, along
//! with the `Math` and `Object` statics, and covers a chain for free: the
//! receiver of a call is itself a candidate, so
//! `["1px","solid"].concat(["red"]).join(" ")` is printed and evaluated once
//! instead of being split across two tables that have to agree.
//!
//! Everything therefore rests on the guard in front of the engine. What it
//! lets through is answered by the language; what it holds back is a boundary
//! with a reason, and each one is named where it is applied.
//!
//! The guard asks whether every leaf of an expression *resolves to a value the
//! bridge can carry*, not whether it is written out — so giving a value a name
//! does not change whether the call on it folds. A name the guard resolved
//! becomes a parameter of a printed arrow and its value an argument to it, so
//! `s.toLowerCase()` is handed over as `(s) => s.toLowerCase()` called with the
//! string `s` holds. See [`Transport`](transport::Transport) for why the value
//! travels beside the source rather than inside it.
//!
//! A name holding a *function* crosses by the other route, because a function
//! has no value an argument could carry: the declaration it came from is
//! printed as the parameter's default and nothing is passed for it. So a
//! callback given a name folds exactly as the same arrow written out in place
//! does, down to which spelling shadows which. Calling such a name is the same
//! carriage and a different question — see [`Position`] for the one thing on
//! this bridge that reads where a call sits.
//!
//! A fold answers one of two things: the value, or the rule that refused it.
//! There is no silent refusal — a call the guard recognised and declined says
//! which rule declined it, rather than falling through to the caller's
//! `Unsupported expression: CallExpression`. A call the guard never recognised
//! is not a refusal at all: it is simply not this module's, and the dispatch
//! below it decides what happens instead.
//!
//! The fold is four concerns and a module each, so the guard — the subject of
//! everything above — reads on its own rather than inside the whole bridge:
//! [`guard`] walks the expression and applies every boundary named here,
//! [`transport`] carries the values it resolved inward, [`amplification`]
//! answers how much a call would build, and [`outward`] reads the engine's
//! answer back.
//!
//! Owning the engine is a fifth and lives in [`engine`]: which expressions may
//! fold is decided in the guard, where they run is decided there, and nothing in
//! that answer turns on a rule the guard applies.
//!
//! What is left here is the way in — [`try_fold`] — and the vocabulary the four
//! share: the refusal, the depth budget and the two allocation ceilings.

mod amplification;
mod backstop;
mod engine;
mod guard;
mod math;
mod outward;
mod theme;
mod transport;

use engine::{ENGINE, Engine, FoldKey, Printed, print_fold, threw};
use guard::{AdmittedKind, Guard, Position, Reader, Repeats, Scope, Walk, admit_an_applied_global};
use outward::Outward;
use theme::is_a_var_group;

pub(super) use guard::unshadowed_applied_global;

/// Whether folding `expr` would print one of the fold's own checks into the
/// source it hands the engine.
///
/// The rewrite [`backstop::checked`] runs, asked of the expression on its own
/// — so it answers for the call a fold prints and not for a declaration that
/// would cross as a parameter default beside it.
///
/// Public and not `#[cfg(test)]`, because the caller is the fold benchmark and
/// a benchmark is a separate crate: a `cfg` set while compiling this one is not
/// set while compiling that. A leg that priced a check and stopped printing one
/// would get quicker and read as a win, which `guidelines/PERFORMANCE.md`
/// refuses. `doc(hidden)` keeps it out of the documented interface, because
/// answering a benchmark is not part of what this crate offers.
#[doc(hidden)]
pub fn prints_a_check(expr: &Expr) -> bool {
  backstop::checked(&mut expr.clone())
}

// Read by the evaluator's own tests, which sit one level up.
#[cfg(test)]
pub(super) use engine::{
  MAX_COMPILED_SCRIPTS, compiled_expressions, forget_engine, holds_a_global, holds_an_engine,
};

use std::borrow::Cow;

use boa_engine::JsValue;
use swc_core::{
  atoms::Atom,
  ecma::ast::{CallExpr, Expr, Lit, MemberProp},
};

use crate::growable_stack;
use stylex_constants::constants::evaluation_errors::{
  BLOCKED_PROPERTY_ACCESS, escaping_property, expression_too_deep, nesting_too_deep_to_carry,
  uncallable_printed_fold,
};

use crate::state::EvaluationState;
use stylex_state::{
  evaluate_result_value::EvaluateResultValue, functions::FunctionMap, state_manager::StateManager,
};

/// What one fold may allocate, in the two units that cost separately.
///
/// The engine bounds recursion and VM stack depth by default, and this module
/// bounds loop iterations itself
/// ([`MAX_LOOP_ITERATIONS`](engine::MAX_LOOP_ITERATIONS)), because Boa's own
/// default for that is `u64::MAX`. What none of them bound is allocation:
/// growth inside a native builtin is not a counted loop. So
/// `"x".repeat(200000000)` is a typo that folds — agreeing with the language —
/// at gigabytes of resident memory, and a compiler that dies there is worse than
/// one that declines the fold.
///
/// Two numbers rather than one, because a value costs in two ways that do not
/// stand in for each other: a bounded string can still become one element per
/// code unit, which costs far more as a tree than it did as text, and a thousand
/// empty arrays hold no text at all and are still a thousand values to build.
///
/// Carried rather than read at each site that spends them, for the reason the
/// depth ceiling is carried: the same number bounds both directions, and sites
/// that each reached for the option could come to disagree about which number
/// that is. The two directions spend them through a [`Totals`] each, and what
/// the guard reads directly is only how much a call *would* build. Where the
/// values come from, and what each costs, is
/// `stylex_structures::fold_ceilings`.
#[derive(Clone, Copy)]
struct Ceilings {
  /// UTF-16 code units of string — the unit the engine's own strings are
  /// measured in.
  characters: u64,
  /// Array elements and object properties.
  entries: u64,
}

/// How much one direction of the bridge has already promised to build, and the
/// ceilings it is promising against.
///
/// A running total rather than a check per value, because what is about to be
/// built is all of them: eight values each one unit under the limit is eight
/// times the limit, and a per-value check waves every one of them through. The
/// shape that made it necessary is aliasing — an engine array holding the same
/// ten-thousand-element array ten thousand times costs the engine one array and
/// costs this side a hundred million syntax nodes, and no single value in it is
/// over the line.
///
/// Both directions count, because both allocate: a resolved name is copied into
/// the engine, and an answer is copied back out as a tree. Each direction keeps
/// its own total, since neither pays for what the other built.
///
/// Two counts, because a value costs in two ways that do not stand in for each
/// other. A thousand empty arrays hold no text at all and are still a thousand
/// values to build; a single string is one entry and can be a megabyte.
struct Totals {
  /// UTF-16 code units of every string and key so far.
  units: u64,
  /// Array elements and object properties so far.
  entries: u64,
  ceilings: Ceilings,
}

impl Totals {
  /// An empty total, counted against `ceilings`.
  fn new(ceilings: Ceilings) -> Self {
    Self {
      units: 0,
      entries: 0,
      ceilings,
    }
  }

  /// Counts `units` code units of string, answering the ceiling it passed where
  /// the running total passed one.
  ///
  /// The ceiling comes back rather than being read off the field, so the sentence
  /// a caller writes names the number this counted against rather than reaching
  /// for it a second time.
  ///
  /// Saturating because the sum exists to be refused on, and a wrapped one would
  /// admit.
  fn count_characters(&mut self, units: u64) -> Result<(), u64> {
    self.units = self.units.saturating_add(units);

    match self.units > self.ceilings.characters {
      true => Err(self.ceilings.characters),
      false => Ok(()),
    }
  }

  /// The same for array elements and object properties.
  ///
  /// Counted before they are walked, so a value past the bound refuses without
  /// first building every entry in it.
  fn count_entries(&mut self, entries: u64) -> Result<(), u64> {
    self.entries = self.entries.saturating_add(entries);

    match self.entries > self.ceilings.entries {
      true => Err(self.ceilings.entries),
      false => Ok(()),
    }
  }
}

/// Whether `name` is one of the names in `list`.
///
/// Every list it is asked about — the guard's two and amplification's two — is
/// three to ten entries, so a scan beats any structure that would have to hash
/// first, and one function keeps every call site from spelling the double
/// reference its own way. Takes a `&str` so a name read off a member property
/// and one read out of a string literal ask the same question rather than one of
/// them reaching past the helper.
fn lists(list: &[&str], name: &str) -> bool {
  list.contains(&name)
}

/// Property names that walk off the value that was written and onto the
/// language's function graph.
///
/// `constructor` on a literal is `String`, whose own `constructor` is
/// `Function`, which compiles a string into a function. So
/// `"".constructor.constructor("return Date.now()").call()` is a chain every
/// other rule admits — two named property reads on a literal, then a call —
/// whose answer is a different number on every build, and whose body can assign
/// to `String.prototype` in an engine every later fold shares. `call`, `apply`
/// and `bind` are what turn an unapplied function back into a call, so they are
/// refused with it.
///
/// Four names: the reads that reach a callable in one step. `__proto__` and
/// `prototype` reach the same graph one step earlier and are refused by
/// [`BLOCKED_PROPERTIES`] instead, which is asked second so that
/// `constructor` keeps the sentence written for it here.
pub(super) const ESCAPING_PROPERTIES: [&str; 4] = ["constructor", "call", "apply", "bind"];

/// Property names that step onto the prototype chain of the value that was
/// written.
///
/// `({}).__proto__` is `Object.prototype`, whose `constructor` is `Object`,
/// whose `constructor` is `Function`. Refusing the first step closes the chain
/// at its root rather than at the callable it ends in, and it makes the
/// compiler answer these reads the way the reference implementation does.
///
/// `constructor` is in this set and in [`ESCAPING_PROPERTIES`]. Escaping is
/// asked first at every call site, so a read of `constructor` keeps the
/// sentence it already had and no author sees a message change. In practice
/// this set is reached by `__proto__` and `prototype` only.
///
/// This is about a property *read*. `__proto__` written as an object-literal
/// key is a different thing -- it sets a prototype rather than reading one --
/// and it folds here as it folds in the reference implementation.
pub(super) const BLOCKED_PROPERTIES: [&str; 3] = ["constructor", "__proto__", "prototype"];

/// The property name a read spells, or `None` where the syntax spells no name.
///
/// A key written as a string is answered too, because `x['constructor']` spells
/// the read `x.constructor` spells. A private name is grammatical only inside a
/// class body, which no value a fold carries has.
///
/// A key the syntax does not spell out is not a name this reader can answer,
/// and it is not therefore a key no rule reads: the dispatch below the fold
/// asks the rules a second time, on the name the key *resolves* to. This
/// reader is the syntactic half of the question only.
pub(super) fn member_prop_name(prop: &MemberProp) -> Option<&str> {
  match prop {
    MemberProp::Ident(name) => Some(name.sym.as_str()),
    MemberProp::Computed(key) => match key.expr.as_ref() {
      Expr::Lit(Lit::Str(text)) => text.value.as_str(),
      _ => None,
    },
    MemberProp::PrivateName(_) => None,
  }
}

/// Whether a property *name* steps onto the prototype chain.
///
/// The membership on its own, for the reader that holds a key it has already
/// resolved and coerced rather than a piece of syntax.
pub(super) fn is_a_blocked_property(name: &str) -> bool {
  lists(&BLOCKED_PROPERTIES, name)
}

/// The words a read of `name` is refused with, or `None` where the name is one
/// a fold may read.
///
/// The two property rules in the order they are asked, in one place. Escaping
/// is asked first so that `constructor` -- which both sets hold -- keeps the
/// sentence written for it, and a third rule would be added here rather than at
/// each of the four sites that ask.
///
/// Shared with the dispatch below the fold, because a read of one of these
/// names is refused whether or not a call is around it: `s.constructor.name` is
/// no more foldable than `s.constructor.constructor('…')()`, and an author who
/// wrote the first would otherwise be told only that a property could not be
/// determined.
pub(super) fn refusal_for_a_property_name(name: &str) -> Option<Refusal> {
  if lists(&ESCAPING_PROPERTIES, name) {
    return Some(escaping_property(name).into());
  }

  if is_a_blocked_property(name) {
    return Some(BLOCKED_PROPERTY_ACCESS.into());
  }

  None
}

/// The same, for a read that is still syntax rather than a resolved name.
///
/// It reads the property one time, where asking the two rules separately reads
/// it two times.
pub(super) fn refusal_for_a_property_read(prop: &MemberProp) -> Option<Refusal> {
  member_prop_name(prop).and_then(refusal_for_a_property_name)
}

/// The words a [refused fold](../../../../../CONTEXT.md) hands the caller,
/// ready to be raised as a deopt.
///
/// Borrowed where the rule has one fixed sentence and owned where it names the
/// method or the limit it refused on, so the common path allocates nothing.
pub(crate) type Refusal = Cow<'static, str>;

/// Why the guard did not hand a call to the engine — the outcome, where
/// [`Refusal`] is the half of it an author reads.
///
/// Only one of the two arms is a refusal in the glossary's sense. A call that
/// is not a candidate was decided from syntax before any rule ran: nothing
/// declined it, so nothing is reported and the dispatch below this module owns
/// the call. A rule that fired declined a call the guard did recognise, and
/// carries the words for it.
enum Decline {
  NotACandidate,
  Rule(Refusal),
}

impl Decline {
  fn rule(reason: impl Into<Refusal>) -> Self {
    Self::Rule(reason.into())
  }
}

/// How much nesting is left, and the one refusal spent at the bottom of it.
///
/// Every walk across the bridge counts the same budget for the same reason ---
/// the guard's walk in, the conversion of a value it resolved, and the
/// conversion out all recurse on the stack — so they share the counter and
/// the sentence rather than keeping three that could drift apart.
///
/// The budget is the project's configured evaluation depth, which is also what
/// the evaluator's own descent spends. One number, because both walks run on
/// the same grown stack for the same reason, and two would mean an author who
/// raised the configured depth still met a ceiling nobody set.
///
/// Reaching the bottom is a refusal and not a "not mine": the shape is one an
/// author can shorten, and saying so is more use than handing it back to a path
/// that would refuse it in vaguer words.
#[derive(Clone, Copy)]
struct Depth {
  /// Levels left before the walk refuses.
  left: usize,
  /// The ceiling the refusal names — kept beside the count because a walk
  /// that has spent its budget no longer knows what it started with.
  ceiling: usize,
}

impl Depth {
  /// A full budget, at the start of a walk or a conversion.
  fn full(ceiling: usize) -> Self {
    Self {
      left: ceiling,
      ceiling,
    }
  }

  /// A full budget again, for a walk that starts where this one stands.
  ///
  /// A value the guard resolved and a value the engine answered are each
  /// converted by a walk of their own, and neither is nested inside the
  /// expression that reached them — so each is measured against the whole
  /// ceiling rather than against what the walk before it had left.
  fn restart(self) -> Self {
    Self::full(self.ceiling)
  }

  /// How many levels the walk has spent to stand where it stands.
  ///
  /// Read only to place an operand the walk does not enter: the nesting of that
  /// operand is measured from itself, and the text around it nests as deep as
  /// the walk that reached it.
  fn spent(self) -> usize {
    self.ceiling - self.left
  }

  /// One level in, or the depth refusal at the bound — so depth is answered
  /// the same way as any other rule the guard applies.
  fn descend(self) -> Result<Self, Decline> {
    match self.left {
      0 => Err(Decline::rule(expression_too_deep(self.ceiling))),
      left => Ok(Self {
        left: left - 1,
        ..self
      }),
    }
  }
}

/// Folds `call` through the engine.
///
/// `None` leaves the existing path in charge: the call is not one this module
/// handles, which is a question of syntax and of what the module's names hold,
/// not a refusal. `Some` is the fold's own answer — the value, or the rule that
/// declined it.
pub(crate) fn try_fold(
  call: &CallExpr,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Option<Result<EvaluateResultValue, Refusal>> {
  let ceiling = traversal_state.evaluation_ceiling();
  let ceilings = Ceilings {
    characters: traversal_state.character_ceiling() as u64,
    entries: traversal_state.entry_ceiling() as u64,
  };

  let guard = Guard {
    scope: Scope::Module,
    depth: Depth::full(ceiling),
    ceilings,
    // The expression the caller asked about is evaluated once.
    repeats: Repeats::Times(1),
    callback: None,
  };

  let mut reader = Reader::new(state, traversal_state, fns, ceilings);
  let mut walk = Walk::new(guard, &mut reader);

  match fold(call, &mut walk) {
    Ok(value) => Some(Ok(value)),
    Err(Decline::NotACandidate) => None,
    Err(Decline::Rule(reason)) => Some(Err(reason)),
  }
}

fn fold(call: &CallExpr, walk: &mut Walk) -> Result<EvaluateResultValue, Decline> {
  let admitted = walk.admit_call(call, Position::Outermost)?;

  let method = admitted.name;

  // How deep the source below is about to nest, which is not how deep the walk
  // above went: an operand a short circuit never reaches is printed and parsed
  // whole without a level ever being spent on it. Past what a stack can be
  // claimed for, the fold refuses — the one answer that is neither an abort nor
  // a wrong one.
  let nesting = walk.printed_nesting();

  if !growable_stack::carriable(nesting) {
    return Err(Decline::rule(nesting_too_deep_to_carry(
      growable_stack::DEEPEST_CARRIED,
    )));
  }

  // Claimed here rather than around the whole fold. Above this line is the
  // guard's walk, which is this compiler's own recursion and asks for room at
  // every level it spends; below it is work that cannot ask — SWC's printer
  // clones the expression and writes it out, and the engine's parser reads it
  // back, and both descend through a nested literal on whatever stack they were
  // handed. So a call the guard declined never pays for a stack the engine never
  // entered, and everything that does need one runs inside it.
  growable_stack::grown_for_depth(nesting, || {
    // The key rather than the printed source: what the engine is asked for is
    // an expression it may already hold a compiled script for, and printing one
    // to find that out is most of what a hit would cost.
    let key = FoldKey::new(call, walk.parameters_key());

    ENGINE.with_borrow_mut(|slot| {
      // Taken, not borrowed in place. A panic unwinding out of the engine is
      // caught at the NAPI boundary and the process carries on, so an engine left
      // in the slot would be reused by every later fold with its VM stack
      // abandoned mid-frame. Taking it means an unwind leaves the slot empty and
      // the next fold builds a fresh engine; the abandoned one leaks, which is
      // what `ManuallyDrop` already makes it do at thread exit.
      let taken = match slot.take() {
        Some(engine) => Ok(engine),
        None => Engine::new(),
      };

      // Read through the answer rather than out of it, so the engine's own
      // refusal is the fold's refusal without a second place that carries it.
      //
      // Written this way for the coverage gate. `Engine::new` builds from the
      // prelude and the trap source this compiler ships, so it cannot fail from
      // anything a module reaches it with, and a `?` here leaves a region no
      // case can enter. A step that *can* fail is asked of
      // [`Engine::started_on`] instead, which takes both sources so a case can
      // hand in one that fails. Nothing is swallowed: the failure still leaves
      // through this closure's `Result`.
      taken.and_then(|mut engine| {
        // Cloned rather than borrowed: it is a handle the engine's own collector
        // owns, and the build below needs the engine's context borrowed at the
        // same time.
        let var_group = engine.var_group.clone();

        let depth = walk.guard.depth.restart();
        let mut outward = Outward::new(method, walk.guard.ceilings);

        let applied = match admitted.kind {
          AdmittedKind::Global => admit_an_applied_global(method, &mut engine.context),
          AdmittedKind::Method | AdmittedKind::Named => Ok(()),
        };

        let folded = applied
          .and_then(|()| walk.arguments(&mut engine.context, method, &var_group))
          .and_then(|arguments| {
            apply(
              key,
              || print_fold(call, walk.parameters()),
              &arguments,
              &mut engine,
              method,
            )
          })
          // Whether the answer is the theme group itself.
          //
          // Asked only where a group crossed, so an ordinary answer pays
          // nothing for a question that could not be true of it.
          //
          // The read carries its refusal rather than reading a throw as "not
          // a group". Reading it as "not a group" was wrong: the walk out
          // below reads *own* keys, so a marker on the answer's prototype is
          // never read again and the throw is never met a second time. The
          // answer would fold to an empty object and the declaration would be
          // dropped with nothing said.
          //
          // No source builds such an answer now -- the statics that put a
          // marker on a prototype are outside the allowlist, in a callback
          // body as much as written out -- so the throw itself is asked of the
          // marker reader directly, in `var_group_tests`. The refusal travels
          // through the chain rather than through a `?`, because a `?` here is
          // a branch of this function that no source can take.
          .and_then(|value| match walk.carried_a_theme_reference() {
            true => {
              is_a_var_group(&value, method, &mut engine.context).map(|group| (value, group))
            },
            false => Ok((value, false)),
          })
          .and_then(|(value, is_the_group)| match is_the_group {
            // A group standing where a value belongs is handed back rather
            // than converted: the group's members live in another file and
            // nothing this side can write stands for it, where the dispatch
            // below holds the reference and answers for it. A refusal here
            // would fail a build it can compile.
            true => Err(Decline::NotACandidate),
            false => outward.value(&value, &mut engine.context, depth),
          });

        *slot = Some(engine);

        folded
      })
    })
  })
}

/// Evaluates the printed expression and, where there is something to pass,
/// calls what it evaluated to with the transported values.
///
/// Two steps rather than one *when there is something to pass*, because the
/// values cross as arguments rather than as text. Every step can throw and all
/// of them are answered the same way: a throw is an answer, not a failure of
/// this module — the language throws on `[].reduce(f)` too — so the engine's own
/// sentence is what the author reads rather than a generic refusal standing in
/// for it.
///
/// A fold that resolved no name and binds neither of the fold's two checks is
/// evaluated directly, which is the whole of why this branches. Wrapping it in
/// an arrow and invoking that arrow costs a function object and a VM frame on
/// top of the expression itself: measured, it
/// is +44% on the cheapest leg of the benchmark and +24% on the chain, paid by
/// exactly the folds that gained nothing from the transport, since every
/// expression that folded before this work resolves no name. The branch is on one
/// question — did the guard resolve anything — and both arms hand the same
/// expression to the same engine, so it is not the two-tables-that-must-agree
/// this module exists to remove.
///
/// Both arms go through the memo, because what is memoised is the compiled
/// script and neither arm has to be a function for that.
///
/// The compiled value is a function by construction, so the refusal for one that
/// is not stands in for a broken invariant rather than for anything an author can
/// write. It is a refusal all the same: this runs inside an evaluation whose
/// whole contract is that it may fail, where an assertion would abort a build
/// that a deopt would only leave to the runtime.
fn apply(
  key: FoldKey,
  print: impl FnOnce() -> Printed,
  arguments: &[JsValue],
  engine: &mut Engine,
  method: &Atom,
) -> Result<JsValue, Decline> {
  let (evaluated, binds_the_checks) = engine.eval(key, print, method)?;

  if !binds_the_checks && arguments.is_empty() {
    return Ok(evaluated);
  }

  let Some(callable) = evaluated.as_callable() else {
    return Err(Decline::rule(uncallable_printed_fold(method)));
  };

  // The two checks are the first parameters of the arrow that binds them, so
  // they are the first arguments too. Borrowed where the fold needs neither, so
  // the common call copies nothing.
  let passed: Cow<'_, [JsValue]> = match binds_the_checks {
    true => Cow::Owned(
      engine
        .backstops
        .arguments()
        .chain(arguments.iter().cloned())
        .collect(),
    ),
    false => Cow::Borrowed(arguments),
  };

  callable
    .call(&JsValue::undefined(), &passed, &mut engine.context)
    .map_err(|error| threw(method, &error))
}

// Read by the two suites that ask the engine what a value they built behaves
// like, which sit one level down beside the modules that build them.
#[cfg(test)]
#[path = "tests/engine_reads.rs"]
mod engine_reads;

#[cfg(test)]
#[path = "tests/property_read_rules_tests.rs"]
mod property_read_rules_tests;

// What the printed expression comes to once the carried values are passed to
// it, asked directly because both refusals are shapes no source prints.
#[cfg(test)]
#[path = "tests/applied_fold_tests.rs"]
mod applied_fold_tests;

// The two callback parameters that bind nothing, which no source writes.
#[cfg(test)]
#[path = "tests/unwritable_pattern_tests.rs"]
mod unwritable_pattern_tests;

// The prototype-chain rule, asked over the spellings an escape is written with.
#[cfg(test)]
#[path = "tests/prototype_chain_escape_tests.rs"]
mod prototype_chain_escape_tests;
