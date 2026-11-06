//! One JavaScript engine per thread, and the printed source it is handed.
//!
//! Owning the engine is a concern of its own, separate from deciding what may
//! reach it. The [guard](super::guard) answers which expressions fold; this answers
//! where they run, how long the engine lives, and what reading a value out of
//! one costs — none of which turns on a rule the guard applies.
//!
//! What it offers the rest of the fold is small, and deliberately: build or
//! reuse the thread's engine, evaluate a printed expression on it, read a value
//! back with a throw named for the method that raised it, and print a call as
//! the source the engine parses.

use std::{cell::RefCell, mem::ManuallyDrop};

use boa_engine::{
  Context, JsError, JsResult, JsValue, Script, Source, object::builtins::JsFunction,
};
use rustc_hash::FxHashMap;
use swc_core::{
  atoms::Atom,
  common::DUMMY_SP,
  ecma::{
    ast::{CallExpr, Expr, ExprStmt, Module, ModuleItem, Pat, Stmt},
    codegen::Config,
  },
};

use stylex_ast::ast::factories::{
  create_arrow_expression_with_params, create_binding_ident, create_ident,
};
use stylex_constants::constants::evaluation_errors::{engine_did_not_start, engine_threw};
use stylex_utils::hash::stable_hash_unspanned_call;

use super::Decline;
use super::backstop::{self, BACKSTOP_SOURCE, Backstops};
use super::math;
use super::theme::{compile_traps, var_group_traps};
use stylex_diagnostics::code_frame::print_module;

/// How many loop iterations an evaluation may run.
///
/// Boa's own default is `u64::MAX` — `RuntimeLimits::loop_iteration` documents
/// it as no limit — so a loop is unbounded until it is said to be bounded. The
/// guard refuses the shapes that reach a loop body, and this is the second
/// answer behind that one: a bound the engine enforces whatever the guard let
/// through. Ten million iterations of an empty loop is well under a second, and
/// no folded CSS value is reached by counting that far.
pub(super) const MAX_LOOP_ITERATIONS: u64 = 10_000_000;

/// What the engine answers when anything asks a function for its source text.
///
/// A function's `ToString` is its source, and this compiler has none to give: an
/// arrow reaches the engine as this module's own minified printing of it, not as
/// the text the author wrote, so `String(() => 'x')` would fold to a spelling no
/// other build produces — and a class name is a hash of the declaration text.
/// The reference compiler answers such a call with the source of a wrapper from
/// inside its own evaluator, which is no better.
///
/// So the source is taken away and every conversion that would read one throws,
/// which the fold reports as a refusal. A function reached only to be *called*
/// is untouched, which is what `String({ toString: () => 'red' })` needs, and is
/// the whole of the difference between a function used as a value and one used
/// as a method.
///
/// Assigned rather than defined, because `Function.prototype.toString` is a
/// writable property; and assigned once when the engine is built, because a fold
/// cannot reach it afterwards — every route from a value to a prototype is a
/// refused property read.
const NO_FUNCTION_SOURCE: &str = concat!(
  "Function.prototype.toString = function () {",
  " throw new TypeError('A function has no source text at compile time.')",
  " };"
);

thread_local! {
  /// One engine per thread, created on the first fold that needs it and reused
  /// for every later one. A file with no foldable method call never builds it.
  ///
  /// Reuse is what makes the guard load-bearing rather than merely tidy: a fold
  /// that reached a prototype would be read by every later fold in the build,
  /// including one in another file, so the boundaries the [guard](super::guard)
  /// applies are what keeps one engine safe to share. Reuse also costs: the
  /// engine interns each distinct source it is handed and never reclaims it,
  /// measured at roughly half a kilobyte per distinct folded call site, which a
  /// real corpus keeps in the low megabytes for the life of the process.
  ///
  /// One per thread and never one more: the napi binding is synchronous and
  /// takes an `Env`, so it folds on the JavaScript thread that called it, and
  /// the threads a build has are the ones its host already had. That is what
  /// makes the leak below a cost per *thread* rather than per file — how many
  /// threads there are, and how often a host retires one, are the host's answer
  /// rather than this compiler's.
  /// `docs/adr/0008-the-fold-guard-reads-values-and-the-engine-is-permanent.md`
  /// carries the argument, and `thread_isolation_tests` the observation.
  ///
  /// `ManuallyDrop` is not a convenience: the engine's garbage collector lives
  /// in a thread-local of its own, and the order two thread-locals are dropped
  /// in is not defined. Dropping this one after the collector's underflows a
  /// reference count, and that panic runs inside a destructor, which aborts the
  /// process instead of unwinding. Leaking one engine per thread at exit is the
  /// price of not aborting. It wraps the whole of [`Engine`] rather than the
  /// context alone, because the memo beside it holds engine values and dropping
  /// one of those late underflows the same count.
  pub(super) static ENGINE: RefCell<Option<ManuallyDrop<Engine>>> = const { RefCell::new(None) };
}

/// How many compiled scripts one thread's memo holds before it is emptied.
///
/// A compiled script is roughly half a kilobyte of bytecode, so this is a
/// ceiling of about a megabyte per thread — and the number is a memory bound
/// rather than a tuning knob: a build with fewer distinct folded call sites than
/// this never reaches it, and one with more only re-parses the shapes it meets
/// after the reset. It is well above the largest real file, which is what keeps
/// the reset off the path any ordinary build takes.
///
/// Emptied rather than evicted one entry at a time. An LRU would carry the
/// hottest shapes across the boundary, and would charge a recency update to
/// every hit to do it — a cost on the path a build spends its time on, to save
/// a re-parse on a path it reaches at most a handful of times.
pub(in crate::evaluate) const MAX_COMPILED_SCRIPTS: usize = 2048;

/// What two folds have to agree on to share one compiled script.
///
/// The printed text was the key until it was this, which meant printing before
/// there was anything to look one up with — so the print, the deep clone of the
/// call and the emitter walk were paid on a hit as well as on a miss. Both
/// halves here are to hand the moment the guard finishes, and both are already
/// 128 bits wide, so the lookup happens first and only a miss prints.
///
/// Hashes rather than the values they stand for, so the memo retains no source
/// text and no expression of its own.
///
/// **The call**, hashed span-insensitively: the same shape written a thousand
/// times in a file is one entry, which is the whole of what the memo is for.
///
/// **The parameters**, because they are printed and the call is not the whole of
/// what is printed: the same call resolves different names in different modules,
/// and a name holding a function is printed with its declaration as a default.
/// Two folds agreeing on the call alone would share a script one of them never
/// wrote.
///
/// Two fields rather than one combined hash, because combining them would be a
/// third hashing pass over two values that are each already wide enough to be
/// read as a key on their own.
///
/// Structure separates a little more than the printed text would: a literal's
/// own spelling is part of the call and not of the minified print, so `'a'` and
/// `"a"` are two entries. That is the safe direction — it costs a parse, where a
/// key blind to the spelling would risk one script standing for two texts the
/// printer quotes differently.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct FoldKey {
  call: u128,
  parameters: u128,
}

impl FoldKey {
  /// The key for `call` printed under a parameter list whose shape hashes to
  /// `parameters` — see [`Transport::parameters_key`](super::transport::Transport::parameters_key)
  /// for what that stands for.
  pub(super) fn new(call: &CallExpr, parameters: u128) -> Self {
    Self {
      call: stable_hash_unspanned_call(call),
      parameters,
    }
  }
}

/// A thread's engine and the [fold memo](../../../../../CONTEXT.md) that may only
/// live as long as it does.
///
/// The two are one value because their lifetimes are one lifetime. A compiled
/// script belongs to a particular engine's realm, so a memo that outlived its
/// engine would hand a later engine a script from a realm that no longer exists,
/// and a memo built before the engine would have nothing to parse against.
/// Holding them together is what makes both impossible to write.
pub(super) struct Engine {
  pub(super) context: Context,
  /// One compiled script per distinct printed expression.
  ///
  /// Keyed by what the printed text is made of — see [`FoldKey`] — because
  /// printing and parsing are both what a warm fold would otherwise repeat: a
  /// file writing one shape a thousand times would print a thousand identical
  /// strings and parse each one. Reuse across files is safe for the reason a
  /// shared engine is — a printed expression carries no name it did not resolve,
  /// since every one the guard resolved became a parameter of an arrow and its
  /// value travels beside it as an argument.
  ///
  /// A compiled script rather than the value or the function it evaluates to,
  /// because those are two shapes and this is one. The bare form evaluates to
  /// the answer and the arrow form to a function still waiting for its
  /// arguments, so a memo of results would have to hold both and a memo of
  /// functions could not hold the first at all — and re-running compiled
  /// bytecode is what both of them wanted anyway.
  ///
  /// It is bounded by [`MAX_COMPILED_SCRIPTS`] rather than by the life of the
  /// thread, which is the difference between a watch-mode process and a
  /// one-shot build: without a bound the memo grows with every distinct call
  /// site every save introduces, for as long as the dev server runs.
  memo: FxHashMap<FoldKey, Compiled>,
  /// What a `defineVars` group crosses as, built once per engine — see
  /// [`theme`](super::theme).
  ///
  /// Kept beside the memo rather than rebuilt per group, and for the same
  /// reason: what it saves is a parse, and the source it parses is the same one
  /// every group in the build reads through.
  pub(super) var_group: JsFunction,
  /// The reader and the caller every printed fold that needs one is handed —
  /// see [`backstop`](super::backstop).
  pub(super) backstops: Backstops,
}

/// A printed expression and what calling it needs.
///
/// The two travel together because the memo answers for both: a hit does not
/// print, so it cannot see whether the arrow it is about to call binds the
/// fold's two checks. Saying it here, once, is what lets every hop between the
/// printer and the call read one name for it rather than a bare `bool`.
pub(super) struct Printed {
  pub(super) source: String,
  /// Whether the arrow takes the fold's two checks as its first parameters.
  pub(super) binds_the_checks: bool,
}

/// A parsed script and what calling it needs, which is [`Printed`] once the
/// source has been through the parser.
#[derive(Clone)]
struct Compiled {
  script: Script,
  binds_the_checks: bool,
}

impl Engine {
  /// A context with the one runtime limit its default leaves open, without the
  /// one thing the language provides that this compiler cannot — function source
  /// text — with the `Math` statics of Node in place of the ones that differ
  /// (see [`math`](super::math)), and with an empty memo.
  ///
  /// Answers a refusal rather than asserting, because the assignment runs inside
  /// an evaluation whose whole contract is that it may fail — and because an
  /// engine that kept function source would fold a spelling no other build
  /// produces, which is worse than declining the fold.
  pub(super) fn new() -> Result<ManuallyDrop<Self>, Decline> {
    Self::started_on(NO_FUNCTION_SOURCE, &var_group_traps(), BACKSTOP_SOURCE)
  }

  /// The same engine, built from three written sources rather than from the
  /// three that are shipped.
  ///
  /// Every step that reads a source refuses, and none can fail for the sources
  /// [`Engine::new`] hands in — two are constants and the third is assembled
  /// from two of the compiler's own constants. So the sources are parameters,
  /// and a case hands in one that fails the step it is about. The refusals stay
  /// because a rename that breaks any of them is declined here rather than
  /// folded past.
  fn started_on(
    prelude: &str,
    traps: &str,
    backstops: &str,
  ) -> Result<ManuallyDrop<Self>, Decline> {
    let mut context = Context::default();

    context
      .runtime_limits_mut()
      .set_loop_iteration_limit(MAX_LOOP_ITERATIONS);

    // The only built-ins that are changed in Rust, not by a source. This reads
    // no source, so it cannot fail and has no refusal.
    math::install(&mut context);

    context
      .eval(Source::from_bytes(prelude))
      .map_err(|error| Decline::rule(engine_did_not_start(&error.to_string())))?;

    let var_group = compile_traps(traps, &mut context)?;
    let backstops = backstop::compile_backstops(backstops, &mut context)?;

    Ok(ManuallyDrop::new(Self {
      context,
      memo: FxHashMap::default(),
      var_group,
      backstops,
    }))
  }

  /// What the expression `key` stands for evaluates to, printing and parsing it
  /// the first time this engine is asked for it and re-running the compiled
  /// script every time after.
  ///
  /// Named after the engine call it stands in for rather than after the memo,
  /// because the memo is what it does and not what it is for.
  ///
  /// This is the engine's own `eval` with the print and the parse lifted out of
  /// it — `eval` is exactly parse-then-evaluate — so what the memo changes is
  /// when a source is written and read, and nothing else about what a fold
  /// answers.
  ///
  /// `print` is taken rather than the text it produces, because producing it is
  /// half of what the memo exists to save: it deep-clones the call, rebuilds the
  /// parameter list, drops every span and runs the emitter, all to reach a
  /// string that on a hit nothing reads.
  ///
  /// The print and the parse are what the memo saves; the evaluation is not
  /// saved and must not be. Re-running is what keeps a fold answering its own
  /// value rather than the first caller's — an expression that mutates a literal
  /// it built reorders a fresh array on every run, and an arrow form evaluates
  /// to a function still waiting for the arguments this fold is about to pass
  /// it.
  ///
  /// A source the parser refused is never memoised, so a later fold of the same
  /// shape is refused by the parser again rather than by a cached mistake. It is
  /// a refusal rather than an assertion because this runs inside an evaluation
  /// whose whole contract is that it may fail, where an assertion would abort a
  /// build that a deopt would only leave to the runtime.
  pub(super) fn eval(
    &mut self,
    key: FoldKey,
    print: impl FnOnce() -> Printed,
    method: &Atom,
  ) -> Result<(JsValue, bool), Decline> {
    let compiled = match self.memo.get(&key) {
      Some(compiled) => compiled.clone(),
      None => self.compile(key, print(), method)?,
    };

    let value = compiled
      .script
      .evaluate(&mut self.context)
      .map_err(|error| threw(method, &error))?;

    Ok((value, compiled.binds_the_checks))
  }

  /// Parses `source` and records it under `key`, emptying the memo first where
  /// recording it would take the thread past [`MAX_COMPILED_SCRIPTS`].
  ///
  /// Emptied before the insert rather than after, so the map is never larger
  /// than the bound rather than one entry larger than it.
  fn compile(
    &mut self,
    key: FoldKey,
    printed: Printed,
    method: &Atom,
  ) -> Result<Compiled, Decline> {
    let script = Script::parse(Source::from_bytes(&printed.source), None, &mut self.context)
      .map_err(|error| threw(method, &error))?;

    let compiled = Compiled {
      script,
      binds_the_checks: printed.binds_the_checks,
    };

    if self.memo.len() >= MAX_COMPILED_SCRIPTS {
      self.memo.clear();
    }

    self.memo.insert(key, compiled.clone());

    Ok(compiled)
  }
}

/// A throw, in the engine's own words under this compiler's naming of the call
/// that produced it.
///
/// Takes the method rather than a direction, because both directions throw: a
/// getter runs while a value is read back out, and a property is written while
/// one is carried in. What an author needs from either is the same two things,
/// and neither of them is which way the value was going.
pub(super) fn threw(method: &Atom, error: &JsError) -> Decline {
  Decline::rule(engine_threw(method, &error.to_string()))
}

/// A read across the bridge, with a throw carried in the engine's words.
///
/// Reading a property runs a getter and writing one can be refused, so both
/// directions need the same answer the evaluation itself gets rather than a
/// second one of their own.
pub(super) fn read<T>(method: &Atom, read: impl FnOnce() -> JsResult<T>) -> Result<T, Decline> {
  read().map_err(|error| threw(method, &error))
}

/// The call as the minified source the engine is handed: an arrow over the names
/// the guard resolved, whose values [`apply`](super::apply) passes to it as
/// arguments — or the call alone where it resolved none.
///
/// The bare form is not a second path so much as the absence of one: an arrow
/// over no parameters, invoked immediately, is the same expression with a
/// function object and a VM frame added, and [`apply`](super::apply) carries the
/// measurement that says what those cost. Printing the call itself is what lets
/// an expression that names nothing pay nothing, and costs the memo nothing
/// either, because what the memo holds is a compiled script rather than a
/// function.
///
/// The module is assembled here rather than by `create_module`, which takes
/// `&Expr` and clones it — so going through it means cloning the subtree once
/// to build the `Expr` and once more inside. The printer needs an owned tree
/// either way, because it drops the spans in place before emitting.
///
/// Called only where [`Engine::eval`] misses its memo, which is what makes the
/// clone and the emitter walk below a per-shape cost rather than a per-fold one.
pub(super) fn print_fold(call: &CallExpr, mut params: Vec<Pat>) -> Printed {
  let mut folded = Expr::Call(call.clone());

  // The call and each declaration crossing as a default are rewritten one tree
  // at a time, because what a tree binds is read from that tree — see
  // [`backstop::checked`]. The arrow below is not one of them: its parameters
  // are the names this compiler carried, not names the source bound.
  let mut binds_the_checks = prepared_for_the_engine(&mut folded);

  for param in &mut params {
    if let Pat::Assign(declared) = param {
      binds_the_checks |= prepared_for_the_engine(&mut declared.right);
    }
  }

  let mut printed = match params.is_empty() {
    true => folded,
    false => create_arrow_expression_with_params(params, folded),
  };

  if binds_the_checks {
    printed = with_the_checks_bound(printed);
  }

  let module = Module {
    span: DUMMY_SP,
    body: vec![ModuleItem::Stmt(Stmt::Expr(ExprStmt {
      span: DUMMY_SP,
      expr: Box::new(printed),
    }))],
    shebang: None,
  };

  let source = print_module(
    module,
    Some(
      Config::default()
        .with_minify(true)
        .with_omit_last_semi(true),
    ),
  );

  Printed {
    source,
    binds_the_checks,
  }
}

/// One tree of the printed source with each `**` changed into a call of the
/// native that computes it as Node does (see [`math`](super::math)), and with
/// the fold's two checks put in. Answers whether the tree needs the checks.
fn prepared_for_the_engine(tree: &mut Expr) -> bool {
  math::exponentiation_as_native_calls(tree);

  backstop::checked(tree)
}

/// The printed expression with the fold's two checks bound in front of whatever
/// else it takes.
///
/// In front, so both are initialised before any parameter default that reads
/// one is evaluated, and in the order the arguments are passed in. An
/// expression that resolved no name has no arrow yet and is given one here,
/// which is the only thing the two shapes differ by.
fn with_the_checks_bound(printed: Expr) -> Expr {
  let checks =
    backstop::reserved_names().map(|name| Pat::Ident(create_binding_ident(create_ident(name))));

  match printed {
    Expr::Arrow(mut arrow) => {
      arrow.params = checks.chain(arrow.params).collect();

      Expr::Arrow(arrow)
    },
    body => create_arrow_expression_with_params(checks.collect(), body),
  }
}

// The readings a case makes of the thread's engine, and the suite whose subject
// is the engine itself. Both sit beside the other suites of this fold rather
// than in this file, because both are scaffolding.
#[cfg(test)]
#[path = "tests/engine_state.rs"]
mod engine_state;

#[cfg(test)]
pub(in crate::evaluate) use engine_state::{
  compiled_expressions, forget_engine, holds_a_global, holds_an_engine,
};

#[cfg(test)]
#[path = "tests/engine_lifetime_tests.rs"]
mod engine_lifetime_tests;
