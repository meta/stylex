//! The walk in front of the engine, and the boundaries it applies.
//!
//! This is the module the fold is named after. It asks whether every leaf of an
//! expression *resolves to a value the bridge can carry*, not whether it is
//! written out — so giving a value a name does not change whether the call on it
//! folds. What it lets through is answered by the language; what it holds back is
//! a boundary with a reason, and each one is named where it is applied.
//!
//! [`Walk`] is what every rule below is written on: the guard it walks under
//! beside the reader that resolves a name to the value it holds. What the guard
//! carries, and the one thing it deliberately does not, is [`Guard`].

use boa_engine::{Context, JsString, JsValue, object::builtins::JsFunction};
use rustc_hash::{FxHashMap, FxHashSet};
use swc_core::{
  atoms::Atom,
  ecma::ast::{
    ArrayLit, ArrowExpr, ArrowFunctionBody, BinExpr, CallExpr, Callee, CondExpr, Decl, Expr,
    ExprOrSpread, ExprStmt, Ident, IdentName, KeyValueProp, Lit, MemberExpr, MemberProp, ObjectLit,
    ObjectPatProp, Pat, Prop, PropName, PropOrSpread, ReturnStmt, Stmt, Tpl,
  },
};

use stylex_ast::ast::convertors::normalize_expr;
use stylex_constants::constants::common::VALUE_ONLY_GLOBALS;
use stylex_constants::constants::evaluation_errors::{
  SPREAD_ELEMENT, amplification_inside_a_callback, global_as_a_value, locale_sensitive_method,
  not_a_function, numeric_literal_receiver, reserved_compiler_name, uncoercible_value,
  unfoldable_function, unfoldable_statement, unfoldable_static,
};
use stylex_enums::declaration_type::DeclarationType;
use stylex_js::coercions::is_global_spelled_as_an_identifier;
use stylex_js::helpers::{is_a_valid_callee_name, is_valid_callee, is_valid_callee_method_name};
use stylex_utils::swc::get_stmt_node_kind;

use super::amplification::EntryAmplifier;
use super::backstop::is_a_reserved_name;
use super::engine::read;
use super::transport::{Crossing, Transport};
use super::{
  Ceilings, Decline, Depth, lists, refusal_for_a_property_name, refusal_for_a_property_read,
};
use crate::growable_stack::{grown_per_level, nesting_of};
use crate::state::EvaluationState;
use stylex_state::{
  evaluate_result_value::EvaluateResultValue, functions::FunctionMap, state_manager::StateManager,
};

use super::super::{
  engine_stylex_functions::{EngineCallable, Reached, engine_callable},
  evaluate_cached,
  helpers::evaluate_result_to_js_boolean,
  nodes::logical_expression::{LogicalOp, evaluates_its_right_operand},
  nodes::member_expression::{get_full_member_path, theme_ref_base},
};

/// Methods whose answer depends on locale data the engine does not carry.
///
/// It resolves them against the root locale, so `"i".toLocaleUpperCase("tr")`
/// comes back `I` where the language says `İ`, and a collation comparison
/// answers by code point. Folding those would put a wrong value in the
/// stylesheet, which is worse than not folding at all.
///
/// `toLocaleString` is here for a receiver the name cannot be separated from.
/// On an object it delegates to `toString` and carries no locale at all, and
/// refusing it costs one of the object methods that would otherwise fold. On a
/// number it formats — `(1234.5).toLocaleString("de-DE")` is `1.234,5` in the
/// language and `1234.5` here — and which receiver a chain will produce is not
/// knowable before evaluating it. One name cannot be both admitted and refused,
/// so it is refused, and the object case is the price.
const LOCALE_SENSITIVE_METHODS: [&str; 4] = [
  "localeCompare",
  "toLocaleLowerCase",
  "toLocaleUpperCase",
  "toLocaleString",
];

/// Where a bare identifier in an expression is allowed to get its value from.
#[derive(Clone, Copy)]
pub(super) enum Scope<'a> {
  /// The module the expression was written in, read through the evaluator. A
  /// name it resolves to a carryable value becomes a parameter of the printed
  /// arrow; a name it cannot is not this module's call.
  Module,
  /// Names a callback binds itself — its parameters, and whatever a block of
  /// its body declares — over the scope they were written inside. The engine
  /// binds them when it runs the callback, so the guard neither resolves one
  /// nor carries a value for it.
  ///
  /// A chain rather than one set, because an inner arrow does not replace the
  /// scope around it: `a.map(x => b.map(y => x + y))` reads `x` from the arrow
  /// outside the one that is being walked.
  Names {
    /// The names this scope introduces, in the order they were written.
    names: &'a [Atom],
    /// Which of them hold an element of a receiver the call around this scope
    /// measured, and how wide such an element is.
    elements: Elements,
    outer: &'a Scope<'a>,
  },
}

impl Scope<'_> {
  /// Whether this scope or any around it binds `name` — in which case the name
  /// is the engine's to resolve, not the module's.
  pub(super) fn binds(&self, name: &Atom) -> bool {
    match self {
      Scope::Module => false,
      Scope::Names { names, outer, .. } => names.contains(name) || outer.binds(name),
    }
  }

  /// Whether the walk is inside a callback body, where a call runs once per
  /// element of a receiver the guard may not have measured.
  pub(super) fn inside_a_callback(&self) -> bool {
    matches!(self, Scope::Names { .. })
  }

  /// What a scope here binds `name` to.
  ///
  /// The innermost scope binding the name answers, so a name shadowing a
  /// measured one is read as itself rather than borrowing its bounds.
  ///
  /// One walk of the chain rather than two. Every reader asked [`Scope::binds`]
  /// and then what the name was bound to, and the second answer already carries
  /// the first — while keeping the two apart, which is what stops a parameter
  /// from being read as the module name it shadows.
  pub(super) fn bound(&self, name: &Atom) -> Bound {
    match self {
      Scope::Module => Bound::NotInScope,
      Scope::Names {
        names,
        elements,
        outer,
      } => match names.iter().position(|bound| bound == name) {
        Some(at) => match elements.holding(at) {
          Some(bounds) => Bound::Measured(bounds),
          None => Bound::Unmeasured,
        },
        None => outer.bound(name),
      },
    }
  }
}

/// What a callback scope binds a name to.
///
/// Three answers rather than two, and the difference between the last two is
/// load-bearing: a name no scope binds is the module's to resolve, and a name a
/// scope binds but nothing measured is bounded by nothing -- which stops the
/// reading rather than sending it to the module, whose value for the same
/// spelling the parameter shadows.
#[derive(Clone, Copy)]
pub(super) enum Bound {
  /// No scope here binds the name, so it is the module's.
  NotInScope,
  /// A scope binds it and nothing measured what it holds.
  Unmeasured,
  /// A scope binds it to a value a call measured for the callback around it.
  Measured(Bounds),
}

/// What the guard read about a value it cannot see, because the engine is what
/// binds it.
///
/// Two readings of one value rather than two values, so a width and a count
/// taken off the same element cannot come to disagree. Both are upper bounds:
/// reading either short admits a call nothing bounded, where reading it long
/// only refuses sooner.
#[derive(Clone, Copy, Default)]
pub(super) struct Bounds {
  /// The characters it renders to under the language's own `ToString`.
  pub(super) characters: Option<u64>,
  /// The largest number it is — set only where the guard could see that it is
  /// a number at all, because a value that merely *coerces* to one is a string
  /// under `+` and would make a sum of bounds no bound.
  pub(super) magnitude: Option<u64>,
}

/// What the guard knows about the values a scope's names hold.
///
/// Two of a callback's parameters are values the call around it measured — the
/// one handed an element of the receiver, and the one handed that element's
/// index. Which position each sits in is the method's to say, so both are
/// recorded as positions in the scope's name list rather than assumed to lead
/// it: `map` hands the element first and `reduce` hands it after an
/// accumulator.
///
/// Every other name — a later parameter, and a name a block of the body
/// declares — holds whatever the body built, which nothing here bounded.
#[derive(Clone, Copy, Default)]
pub(super) struct Elements {
  /// The names handed an element, as the half-open span of the scope's list
  /// the parameter in that position binds. A span rather than one position
  /// because the parameter may destructure the element into several names.
  element_names: (usize, usize),
  /// What one element of the receiver holds.
  element: Bounds,
  /// The name handed the element's index, where the parameter in that position
  /// is a plain name, and what that index holds.
  index: Option<(usize, Bounds)>,
}

impl Elements {
  /// What the name at `at` in the scope's list holds, or `None` where it is not
  /// one of the two the call measured.
  fn holding(&self, at: usize) -> Option<Bounds> {
    let (from, to) = self.element_names;

    match self.index {
      Some((position, index)) if position == at => Some(index),
      _ => match (from..to).contains(&at) {
        true => Some(self.element),
        false => None,
      },
    }
  }

  /// The names `callback` measured, placed at the positions `spans` says each
  /// of the arrow's parameters binds.
  ///
  /// The index is the parameter after the element wherever the element sits,
  /// and is only read where that parameter is a plain name: a destructuring in
  /// its place binds parts of a number, which is nothing.
  fn placed(callback: Callback, params: &[Pat], spans: &[(usize, usize)]) -> Self {
    let index_at = callback.element_at + 1;

    Self {
      element_names: spans.get(callback.element_at).copied().unwrap_or_default(),
      element: callback.element,
      index: match params.get(index_at) {
        Some(Pat::Ident(_)) => spans.get(index_at).map(|(from, _)| (*from, callback.index)),
        _ => None,
      },
    }
  }
}

/// How many times the expression under the walk is evaluated.
///
/// One at module scope. Inside a callback it is the product of every enclosing
/// receiver's element count, so nesting multiplies rather than resets — and a
/// callback over a receiver nothing counted is unmeasured, which is the blanket
/// refusal the two amplification rules used to give every callback body.
#[derive(Clone, Copy, Default)]
pub(super) enum Repeats {
  /// Evaluations the guard counted.
  Times(u64),
  /// A callback over a receiver whose element count nothing here read.
  #[default]
  Unmeasured,
}

impl Repeats {
  /// The count a bound on one evaluation is multiplied by, or the refusal that
  /// there is no such count — which is what both amplification rules do with an
  /// unmeasured callback, so the sentence is written once here.
  ///
  /// `built` is the unit the refusal names, for the reason the message takes one:
  /// a call amplifies in one of the two a fold spends, and the two do not stand
  /// in for each other.
  pub(super) fn counted(self, built: &str, call: &str) -> Result<u64, Decline> {
    match self {
      Self::Times(times) => Ok(times),
      Self::Unmeasured => Err(Decline::rule(amplification_inside_a_callback(built, call))),
    }
  }

  /// The repeats of a body evaluated once per element of a receiver holding
  /// `elements`, which multiplies what is already counted rather than replacing
  /// it — so a callback nested in a callback is the product of both receivers.
  ///
  /// Saturating for the reason the product inside one evaluation saturates: it
  /// exists to be refused on, and a wrapped one would admit.
  pub(super) fn per_element(self, elements: u64) -> Self {
    match self {
      Self::Times(times) => Self::Times(times.saturating_mul(elements)),
      Self::Unmeasured => Self::Unmeasured,
    }
  }
}

/// What the call under the walk measured for an arrow it will run.
///
/// Two positions reach one, and they are measured differently. A callback among
/// the arguments runs once per element of the receiver and is handed that
/// element, so it carries everything below. The callee of a call reached through
/// a name runs once per evaluation of the call itself and is handed the
/// arguments, whose bounds nothing here reads — so it carries the count alone.
///
/// One value rather than a field each on the guard, so everything a body needs —
/// how often it runs, and what the values it is handed hold — is read off the
/// same measurement of the same receiver and cannot come to disagree.
#[derive(Clone, Copy, Default)]
pub(super) struct Callback {
  /// How many times the call will run the body.
  pub(super) repeats: Repeats,
  /// What the widest element of the receiver holds.
  pub(super) element: Bounds,
  /// What that element's index holds, which is a number the receiver's own
  /// length settles.
  pub(super) index: Bounds,
  /// Which of the callback's parameters is handed the element.
  pub(super) element_at: usize,
}

/// What the guard carries as it walks: where a bare identifier may come from,
/// and how much nesting is left before the expression is refused as too deep.
///
/// Nothing here records *where* in the expression the walk is. Every rule below
/// reads the call in front of it and nothing else, so a static, a chain link and
/// the call the caller asked about are all answered the same way — which is what
/// one guard walk is for.
#[derive(Clone, Copy)]
pub(super) struct Guard<'a> {
  pub(super) scope: Scope<'a>,
  pub(super) depth: Depth,
  pub(super) ceilings: Ceilings,
  /// How many times the expression under the walk is evaluated.
  pub(super) repeats: Repeats,
  /// What the call under the walk measured for a callback among its arguments,
  /// and `None` where this position reaches no callback the guard counts.
  ///
  /// Set by a call for its own arguments and dropped everywhere else, so the
  /// arrow that reads it is the one written inside the call that measured it.
  pub(super) callback: Option<Callback>,
}

impl<'a> Guard<'a> {
  /// The guard one level in.
  fn descend(self) -> Result<Self, Decline> {
    Ok(Self {
      depth: self.depth.descend()?,
      ..self
    })
  }

  /// The same remaining depth, with `names` bound over the scope this guard
  /// already carries.
  ///
  /// `elements` says which of the names hold a value the call around them
  /// measured, and `repeats` how many times the scope being entered runs. A
  /// block inside a callback passes neither on: what a block declares is a value
  /// the body built, and the body's own repeats are already counted.
  fn binding<'b>(&'b self, names: &'b [Atom], elements: Elements, repeats: Repeats) -> Guard<'b> {
    Guard {
      scope: Scope::Names {
        names,
        elements,
        outer: &self.scope,
      },
      depth: self.depth,
      ceilings: self.ceilings,
      repeats,
      // The scope being entered is not a call, and only the call an arrow was
      // written inside says how often that arrow runs.
      callback: None,
    }
  }
}

/// What the walk needs that the expression does not carry: the evaluator, so a
/// name can be resolved to the value it holds, and the transport the resolved
/// values are collected into.
pub(super) struct Reader<'a> {
  state: &'a mut EvaluationState,
  traversal_state: &'a mut StateManager,
  fns: &'a FunctionMap,
  transport: Transport,
  /// The dotted paths this walk saw read off a theme group, under the name each
  /// was read through — the prefixes that group has to answer with a stand-in.
  /// See [`var_group_traps`](super::theme).
  ///
  /// Per name rather than per fold, because an expression can read two groups
  /// and a path off one says nothing about the other.
  dotted_prefixes: FxHashMap<Atom, FxHashSet<Atom>>,
  /// How deep the printed source nests through the operand the walk declined to
  /// enter, measured from the top of the expression.
  ///
  /// Per fold rather than per step, for the reason the transport is: what the
  /// claim has to cover is the deepest text anywhere in the call, and a step
  /// that kept its own answer could only report the branch it happened to be on.
  unentered_nesting: usize,
}

impl<'a> Reader<'a> {
  /// A reader over the evaluator, holding an empty transport counted against
  /// `ceilings`.
  ///
  /// The transport is built here rather than beside the reader, because a
  /// transport with no reader to fill it is not a thing the fold has — and one
  /// counted against different ceilings than the walk's would bound the two
  /// directions differently.
  pub(super) fn new(
    state: &'a mut EvaluationState,
    traversal_state: &'a mut StateManager,
    fns: &'a FunctionMap,
    ceilings: Ceilings,
  ) -> Self {
    Self {
      state,
      traversal_state,
      fns,
      transport: Transport::new(ceilings),
      dotted_prefixes: FxHashMap::default(),
      unentered_nesting: 0,
    }
  }

  /// The value `expr` resolves to, or `None` where it resolves to nothing.
  ///
  /// Resolved through the evaluator's own memoised entry point rather than by a
  /// second reading of its own, so a binding this fold reads is the binding
  /// every other position reads — including the disqualifications that live
  /// there: a reassigned binding, one mutated in place, and one read above its
  /// own declaration all answer nothing here because they answer nothing there.
  ///
  /// Takes an expression rather than a name because two questions need it: what
  /// a name holds, and what an amplifying call's count comes to. `2 * 2` is a
  /// count as surely as `4` is, and the evaluator is what already knows so.
  ///
  /// The read is a speculation and is marked as one, so nothing it refuses is
  /// left behind: the evaluation's confidence and deopt are put back, and the
  /// memo withholds the refusal. An expression this module could not read is not
  /// a refusal — the dispatch below owns the call, evaluates the same names
  /// itself, and has to find both the state and the sentence it would have had.
  pub(super) fn resolve(&mut self, expr: &Expr) -> Option<EvaluateResultValue> {
    let Reader {
      state,
      traversal_state,
      fns,
      ..
    } = self;

    // The putting-back is the whole contract, so it belongs to the two states
    // that own the fields rather than to a sequence written out here that a
    // later edit could return past.
    speculate(state, traversal_state, |state, traversal_state| {
      evaluate_cached(expr, state, traversal_state, fns)
    })
  }
}

/// Runs `read` as a [speculative
/// read](../../../../../CONTEXT.md#speculative-read), and puts back everything it
/// refused.
///
/// `None` where the read refused, so a caller cannot mistake a refusal for a
/// value: the confidence that says which it was is gone by the time the caller
/// sees the answer, which is the point.
///
/// The two flags are saved and restored rather than cleared, so a fold reached
/// from inside another speculation stays inside it for as long as that one lasts.
/// Written as one function taking a closure rather than as a save, a call and a
/// restore at the call site, because the restore is the contract and a `return`
/// added between the halves would silently drop it.
fn speculate(
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  read: impl FnOnce(&mut EvaluationState, &mut StateManager) -> Option<EvaluateResultValue>,
) -> Option<EvaluateResultValue> {
  let confident = state.confident;
  let deopt_path = state.deopt_path.take();
  let deopt_reason = state.deopt_reason.take();
  let speculating = traversal_state.speculating;

  traversal_state.speculating = true;

  let read = read(state, traversal_state);

  let value = match state.confident {
    true => read,
    false => None,
  };

  traversal_state.speculating = speculating;
  state.confident = confident;
  state.deopt_path = deopt_path;
  state.deopt_reason = deopt_reason;

  value
}

/// What the guard admitted: the name a refusal or a throw is reported under,
/// beside the shape the call reached its function through.
///
/// A name and a kind rather than a kind carrying a name, because all three
/// kinds carry one and only [`AdmittedKind::Global`] is ever told from the
/// others — so a reader that wants the name cannot be made to answer for a
/// kind it does not care about.
#[derive(Clone, Copy)]
pub(super) struct Admitted<'a> {
  /// The method, global or binding the call names.
  pub(super) name: &'a Atom,
  /// How the call reached the function it applies.
  pub(super) kind: AdmittedKind,
}

/// How a call reached the function it applies.
///
/// The first two are the ways a *native* function is reached: a method on a
/// receiver, and a global applied as a function. They are told apart because
/// only the second can name something that is not a function at all — `Math` is
/// a valid callee because its methods fold, which says nothing about whether the
/// name itself can be applied. The third is the author's own function, reached
/// through the name the module bound it under, which the engine holds because
/// its declaration crossed as one.
#[derive(Clone, Copy)]
pub(super) enum AdmittedKind {
  Method,
  Global,
  Named,
}

impl<'a> Admitted<'a> {
  fn method(name: &'a Atom) -> Self {
    Self {
      name,
      kind: AdmittedKind::Method,
    }
  }

  fn global(name: &'a Atom) -> Self {
    Self {
      name,
      kind: AdmittedKind::Global,
    }
  }

  fn named(name: &'a Atom) -> Self {
    Self {
      name,
      kind: AdmittedKind::Named,
    }
  }
}

/// Where the call under the guard sits, which is the whole of what decides who
/// owns a callee written as a bare name.
///
/// The only thing on this bridge that reads position at all, and it is a
/// parameter rather than something [`Guard`] carries, so the rest of the walk
/// keeps answering the call in front of it and nothing else.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Position {
  /// The call the caller asked about, which the dispatch below still owns.
  Outermost,
  /// A call inside an expression the fold has already claimed.
  Inside,
}

/// What a set of patterns binds, and the expressions those patterns evaluate.
///
/// Both in one walk, because the names have to be in scope before an expression
/// beside them is walked and the two are written into the same pattern.
#[derive(Default)]
struct Bindings<'a> {
  /// The names the patterns introduce.
  names: Vec<Atom>,
  /// The expressions the patterns evaluate where they stand: a default value,
  /// and a computed key.
  evaluates: Vec<&'a Expr>,
}

impl<'a> Bindings<'a> {
  /// Records what `pat` binds and what it evaluates, or declines a pattern that
  /// binds no name this walk can put in scope.
  ///
  /// Nesting is spent here as it is everywhere else on this bridge: a pattern
  /// is printed into the transport with the arrow it belongs to, so a
  /// destructuring nested deeper than the engine's parser can descend has to be
  /// refused before it reaches that parser.
  fn pattern(&mut self, pat: &'a Pat, depth: Depth) -> Result<(), Decline> {
    // Room for the next level asked for at this one, which is what every walk
    // this module owns does: the ceiling is configuration and can be raised far
    // past what a thread has left, and an overflow here aborts a build rather
    // than reporting anything. See `growable_stack`.
    grown_per_level(|| self.nested_pattern(pat, depth))
  }

  /// One pattern, on the room [`pattern`](Bindings::pattern) asked for, and
  /// reached only through it — a direct call would descend on no room at all.
  fn nested_pattern(&mut self, pat: &'a Pat, depth: Depth) -> Result<(), Decline> {
    let inner = depth.descend()?;

    match pat {
      Pat::Ident(ident) => self.bind(&ident.sym)?,
      // A hole binds nothing and skips an element, which the engine does itself.
      Pat::Array(array) => {
        for element in array.elems.iter().flatten() {
          self.pattern(element, inner)?;
        }
      },
      Pat::Object(object) => {
        for prop in &object.props {
          match prop {
            // `{ a }` and `{ a = 1 }`: the key is the name, and the default is
            // an expression beside it.
            ObjectPatProp::Assign(shorthand) => {
              self.bind(&shorthand.key.sym)?;

              if let Some(default) = &shorthand.value {
                self.evaluates.push(default);
              }
            },
            // `{ a: b }`, and `{ [k]: b }` whose key is a value in its own
            // right.
            ObjectPatProp::KeyValue(entry) => {
              if let PropName::Computed(key) = &entry.key {
                self.evaluates.push(&key.expr);
              }

              self.pattern(&entry.value, inner)?;
            },
            ObjectPatProp::Rest(rest) => self.pattern(&rest.arg, inner)?,
          }
        }
      },
      Pat::Rest(rest) => self.pattern(&rest.arg, inner)?,
      Pat::Assign(assign) => {
        self.pattern(&assign.left, inner)?;
        self.evaluates.push(&assign.right);
      },
      // `[o.a] = …` assigns through a member rather than binding a name, so
      // there is nothing to put in scope; an invalid pattern is not a shape to
      // reason about at all.
      Pat::Expr(_) | Pat::Invalid(_) => return Err(Decline::NotACandidate),
    }

    Ok(())
  }

  /// Puts one name in scope, or refuses a name the printed source reserves.
  fn bind(&mut self, name: &Atom) -> Result<(), Decline> {
    refuse_a_reserved_name(name)?;

    self.names.push(name.clone());

    Ok(())
  }

  /// The guard with these names in scope, once the expressions beside them have
  /// been walked.
  ///
  /// The order is the whole of this: a pattern's own expressions are evaluated
  /// where the pattern is, so they are walked with every name already bound —
  /// reading one declared later throws where the language throws, rather than
  /// quietly resolving to a module name the binding shadows. Written once here
  /// because both callers depend on it and neither states it.
  fn enter<'b>(
    &'b self,
    guard: &'b Guard,
    elements: Elements,
    repeats: Repeats,
    reader: &mut Reader,
  ) -> Result<Guard<'b>, Decline> {
    let inner = guard.binding(&self.names, elements, repeats);

    for expr in &self.evaluates {
      Walk {
        guard: inner,
        reader: &mut *reader,
      }
      .admit_value(expr)?;
    }

    Ok(inner)
  }
}

/// Refuses a name the printed fold reserves for a check of its own, or for the
/// native that computes `**`.
///
/// The fold prints its two checks as parameters of the arrow it hands the
/// engine, so any other name of the same spelling would shadow one and the
/// check would run the author's function instead. Two spellings reach a
/// parameter of that arrow — a name the walk binds, and a module name it
/// carries — and both are refused here, so the rule reads as one rule wherever
/// it fires. See [`backstop`](super::backstop). The native of `**` is a global,
/// and the same two spellings would shadow it (see [`math`](super::math)).
fn refuse_a_reserved_name(name: &Atom) -> Result<(), Decline> {
  match is_a_reserved_name(name) {
    true => Err(Decline::rule(reserved_compiler_name(name))),
    false => Ok(()),
  }
}

/// The two things every step of the walk needs: the guard it is walking under,
/// and the reader that resolves a name to the value it holds.
///
/// One receiver rather than two parameters threaded side by side. The guard is
/// copied at every step and the reader is not — it collects the transport as the
/// walk goes — so keeping them apart meant every step's signature spelling the
/// pairing out, and one of them could have been handed a sub-walk's guard beside
/// the caller's reader. [`Depth`] is a field of [`Guard`] for the same reason:
/// one value, read where it is needed rather than carried a second time beside
/// the guard that already holds it.
pub(super) struct Walk<'a, 'r> {
  pub(super) guard: Guard<'a>,
  pub(super) reader: &'a mut Reader<'r>,
}

impl<'a, 'r> Walk<'a, 'r> {
  /// A walk over `reader`, starting under `guard`.
  pub(super) fn new(guard: Guard<'a>, reader: &'a mut Reader<'r>) -> Self {
    Self { guard, reader }
  }

  /// Whether a theme reference crossed inward under any of the names this walk
  /// resolved.
  pub(super) fn carried_a_theme_reference(&self) -> bool {
    self.reader.transport.read_a_theme_reference
  }

  /// How deep the source about to be printed nests.
  ///
  /// The ceiling, which every level the walk entered was spent under, or the
  /// deeper reach of an operand it declined to enter — see
  /// [`never_entered`](Walk::never_entered).
  ///
  /// The outermost call and the arrow the transport wraps it in are a couple of
  /// levels the walk never spent, so this reads a level or two short of the text
  /// at exactly the ceiling. That is what the margin on `BYTES_PER_LEVEL` in
  /// [`growable_stack`](crate::growable_stack) is for, and it is more
  /// than twice what a level costs: the fold nested to the largest configurable
  /// ceiling is pinned in the evaluator's own suite.
  pub(super) fn printed_nesting(&self) -> usize {
    self.guard.depth.ceiling.max(self.reader.unentered_nesting)
  }

  /// The names the walk resolved, as the parameters of the printed arrow.
  pub(super) fn parameters(&self) -> Vec<Pat> {
    self.reader.transport.parameters()
  }

  /// The same parameters as the hash the fold memo is keyed on, answered
  /// without printing them.
  pub(super) fn parameters_key(&self) -> u128 {
    self.reader.transport.parameters_key()
  }

  /// The values it resolved, as the arguments that arrow is called with.
  ///
  /// The group builder is taken rather than reached through the engine, because
  /// what builds a value here is the engine's *context* and the two cannot be
  /// borrowed from it at once.
  pub(super) fn arguments(
    &self,
    engine: &mut Context,
    method: &Atom,
    var_group: &JsFunction,
  ) -> Result<Vec<JsValue>, Decline> {
    self.reader.transport.arguments(
      engine,
      method,
      self.guard.depth,
      var_group,
      &self.reader.dotted_prefixes,
    )
  }
}

impl<'r> Walk<'_, 'r> {
  /// The same reader, walking under `guard`.
  ///
  /// The guard a step walks under is the step's own — one level in, or rebuilt
  /// with a scope or a measurement of its own — where the reader is the one
  /// reader the whole fold collects into. So this is what a sub-walk is, and
  /// there is no way to write one that pairs them wrongly.
  fn under<'w>(&'w mut self, guard: Guard<'w>) -> Walk<'w, 'r> {
    Walk {
      guard,
      reader: &mut *self.reader,
    }
  }

  /// Records how deep `expr` nests, for an operand the walk is not entering.
  ///
  /// A dead operand costs the walk nothing and the print and the parse
  /// everything — the engine decides the short circuit itself, so both descend
  /// through it whole. Measuring it here is what lets the claim be made from the
  /// text rather than from a ceiling the text does not answer to. Placed at the
  /// level the walk stands on, since the brackets above the operand are as much
  /// of the descent as the ones inside it.
  fn never_entered(&mut self, expr: &Expr) {
    let nesting = self.guard.depth.spent() + nesting_of(expr);

    self.reader.unentered_nesting = self.reader.unentered_nesting.max(nesting);
  }

  /// Whether every value `expr` needs is written into it, bound by the guard's
  /// scope, or resolvable from the module — so the printed arrow and the values
  /// beside it are together something the engine can evaluate alone.
  ///
  /// One walk serves both questions this module asks — the receiver's and the
  /// arguments' — so a shape accepted in one position cannot silently be refused
  /// in the other. Deliberately narrow: a shape it does not recognise is not a
  /// candidate.
  fn admit_value(&mut self, expr: &Expr) -> Result<(), Decline> {
    grown_per_level(|| self.nested_value(expr))
  }

  /// One expression, on the room [`admit_value`](Walk::admit_value) asked for,
  /// and reached only through it — a direct call would descend on no room at all.
  fn nested_value(&mut self, expr: &Expr) -> Result<(), Decline> {
    let inner = self.guard.descend()?;

    match expr {
      // A name a callback binds — a parameter of its own, or something a block of
      // its body declares — is bound by the engine when it runs the callback. Any
      // other name is asked of the module, and becomes a parameter of the printed
      // arrow carrying the value it resolved to.
      Expr::Ident(ident) => {
        if self.guard.scope.binds(&ident.sym) {
          return Ok(());
        }

        // A module name of the same spelling would be carried as a parameter of
        // the printed arrow, where the fold's own two checks already stand.
        refuse_a_reserved_name(&ident.sym)?;

        // `undefined`, `NaN` and `Infinity` are values the grammar has no literal
        // for, so an author writes them as names and they reach the guard as
        // names. The engine holds them, so they are printed and the language
        // answers — the same arrangement a global receiver takes, and for the same
        // reason. Only where the module bound nothing of the name, in which case
        // the binding is resolved like any other below.
        if is_global_spelled_as_an_identifier(ident)
          && self.reader.traversal_state.declaration_of(ident).is_none()
        {
          return Ok(());
        }

        // A global the language holds and the bridge cannot carry. Answered from
        // the source, so it sits in front of the resolution with the rule above
        // rather than among the arms below — a name the module bound nothing of
        // has nothing to resolve, and asking anyway would price a walk on an
        // answer already known.
        //
        // Named here because nothing else would: the dispatch below owns the
        // call, resolves the name to nothing and reports a constant the author
        // never wrote, which is a sentence about something else. Both compilers
        // refuse the input either way, so only the wording changes.
        if a_global_written_as_a_value(ident, self.reader.traversal_state) {
          return Err(Decline::rule(global_as_a_value(&ident.sym)));
        }

        let depth = self.guard.depth;

        match self.reader.resolve(expr) {
          Some(value) if is_a_carryable_receiver(&value) => {
            self.reader.transport.bind(&ident.sym, value, depth)
          },
          // Everything left is a function, or is nothing this module answers for.
          resolved => {
            // A name holding a function, which is what the evaluator answers a
            // callback with. There is no value form to carry, so the declaration
            // it came from crosses instead.
            let carries_a_function = matches!(resolved, Some(EvaluateResultValue::Callback(_)));

            // Asked before the declaration is read, which is what keeps a name
            // written twice from owning its declaration twice: the transport
            // holds one value per name, so the crossing already happened.
            if carries_a_function && self.reader.transport.holds(&ident.sym) {
              return Ok(());
            }

            // Owned because the walk below takes the evaluator mutably and the
            // declaration is borrowed out of it. One subtree per name per fold,
            // and the printer would have wanted an owned tree anyway.
            //
            // Read for a function alone. The evaluator answered one by reading
            // this very declaration, so there is always one to read — a function
            // without a declaration falls to the reading below, which finds no
            // function either and hands the call back.
            let declaration = match carries_a_function {
              true => initializer_of(ident, self.reader).cloned(),
              false => None,
            };

            match declaration {
              Some(declaration) => self.under(inner).admit_a_named_function(ident, declaration),
              // A name that resolved to nothing is usually not this module's
              // business — the dispatch below owns the call and answers for it. A
              // function is the exception: nothing below the fold carries one
              // into an evaluation.
              None => match the_module_declares_a_function(ident, self.reader) {
                true => Err(Decline::rule(unfoldable_function(&ident.sym))),
                false => Err(Decline::NotACandidate),
              },
            }
          },
        }
      },
      // A regular expression and a BigInt have no value this evaluator carries,
      // and neither does the reference implementation fold one.
      Expr::Lit(Lit::Regex(_) | Lit::BigInt(_)) => Err(Decline::NotACandidate),
      Expr::Lit(_) => Ok(()),
      Expr::Paren(paren) => self.under(inner).admit_value(&paren.expr),
      Expr::Unary(unary) => self.under(inner).admit_value(&unary.arg),
      // Both operands of an arithmetic or a comparison operator are evaluated, so
      // both are walked. A short-circuiting one evaluates its right operand only
      // where the left one lets it, and the walk stops exactly where the language
      // does — see [`Walk::deciding_value_of`] for when it may tell.
      Expr::Bin(bin) => {
        self.under(inner).admit_value(&bin.left)?;

        match self.right_operand_runs(bin) {
          true => self.under(inner).admit_value(&bin.right),
          false => {
            self.under(inner).never_entered(&bin.right);

            Ok(())
          },
        }
      },
      // The test is evaluated and exactly one arm is, on the same terms.
      Expr::Cond(cond) => {
        self.under(inner).admit_value(&cond.test)?;

        match self.arms_of(cond) {
          Some((taken, dead)) => {
            self.under(inner).never_entered(dead);
            self.under(inner).admit_value(taken)
          },
          // A test whose truthiness this walk cannot read: either arm may be the
          // one the engine evaluates, so both have to carry.
          None => {
            self.under(inner).admit_value(&cond.cons)?;
            self.under(inner).admit_value(&cond.alt)
          },
        }
      },
      // A template literal is written-out syntax whose holes are values in their
      // own right, so it is walked like any other composite and printed back
      // exactly as it was written. A tagged one is a different node and is not
      // here: its tag is a call this module cannot see the body of.
      Expr::Tpl(Tpl { exprs, .. }) => {
        for hole in exprs {
          self.under(inner).admit_value(hole)?;
        }

        Ok(())
      },
      // A property read: `x.length` inside a callback, `({a:1}).a` as a receiver,
      // and `p[0]` on an element a callback was handed.
      //
      // The property is answered before the receiver is walked, because the name
      // alone decides it and the walk now resolves bindings — so a read that no
      // receiver could make safe must not cost a resolution first.
      Expr::Member(member @ MemberExpr { obj, prop, .. }) => {
        if let Some(refusal) = refusal_for_a_property_read(prop) {
          return Err(Decline::Rule(refusal));
        }

        // A computed key is a value in its own right, so it is walked as one. A
        // named read has nothing left to walk — the name is the whole of it, and
        // the rule above is what reads it — and a private name is written only
        // inside a class body, which no expression the walk reaches has.
        //
        // A key whose value the guard cannot read is still admitted, because
        // `['a','b'][i]` and `{a:'X'}[k]` are ordinary and have to keep
        // folding. The rule is applied where the name finally exists instead:
        // the printed source reads such a key through a checked reader rather
        // than through the language's index operator, and calls through a
        // checked caller — see [`backstop`](super::backstop) and ADR 0010.
        if let MemberProp::Computed(key) = prop {
          self.under(inner).admit_value(&key.expr)?;
        }

        // A chain of two or more names read off a theme group is one token
        // rather than a read of a read: `colors.brand.primary` names
        // `brand.primary`. The printed source reads the names one at a time, so
        // the group is told which paths it has to answer with a stand-in for the
        // path so far rather than with the variable that path names.
        //
        // After the property, for the reason the property is answered first: a
        // read no receiver could make safe must not cost a resolution.
        self.record_a_dotted_theme_read(member);

        self.under(inner).admit_value(obj)
      },
      Expr::Array(ArrayLit { elems, .. }) => {
        for elem in elems {
          match elem {
            Some(ExprOrSpread { spread: None, expr }) => self.under(inner).admit_value(expr)?,
            // A hole is `undefined` and a spread needs the scope; both stay out.
            _ => return Err(Decline::NotACandidate),
          }
        }

        Ok(())
      },
      // A key and a value written out, which is a value the engine can be handed
      // whole. `__proto__` is the one key that is not: written as a plain
      // property it sets the prototype rather than a member, so the receiver the
      // engine sees is not the object the source appears to describe. It is left
      // in because the reference implementation folds it identically and because
      // every *read* off the prototype is refused above — the rule that refuses
      // the name is about a read, and this is a write.
      Expr::Object(ObjectLit { props, .. }) => {
        for prop in props {
          let prop = match prop {
            PropOrSpread::Prop(prop) => prop,
            // A spread is a value in its own right and the language does the
            // spreading, so the operand is walked and the printed source keeps the
            // spread exactly as it was written.
            PropOrSpread::Spread(spread) => {
              self.under(inner).admit_value(&spread.expr)?;

              continue;
            },
          };

          let Prop::KeyValue(KeyValueProp { key, value }) = prop.as_ref() else {
            return Err(Decline::NotACandidate);
          };

          if !matches!(
            key,
            PropName::Ident(_) | PropName::Str(_) | PropName::Num(_)
          ) {
            return Err(Decline::NotACandidate);
          }

          self.under(inner).admit_value(value)?;
        }

        Ok(())
      },
      // A chained call: the receiver is itself a fold. Printing the whole chain
      // and evaluating it once is what makes `[…].map(…).join('-')` work, which
      // two separate method tables cannot agree on.
      //
      // A StyleX function is asked about first, because its callee is a name the
      // module bound and every question `admit_call` asks would answer "not mine"
      // for it.
      Expr::Call(call) => {
        let callable = self.under(inner).a_stylex_function(call);

        match callable {
          Some(callable) => self.under(inner).admit_a_stylex_function(&callable, call),
          None => self
            .under(inner)
            .admit_call(call, Position::Inside)
            .map(|_| ()),
        }
      },
      // An arrow is a value the language can hold and call: the callback `map` and
      // `filter` take, and the own `toString` an object converts through. It has
      // no *string* form here — the engine is built without function source text,
      // so a conversion that would read one refuses. See
      // `NO_FUNCTION_SOURCE` in [`engine`](super::engine).
      Expr::Arrow(arrow) => self.under(inner).admit_arrow(arrow),
      _ => Err(Decline::NotACandidate),
    }
  }

  /// Records the paths `member` reads off a theme group, where it reads any.
  ///
  /// The same question the dispatch below asks of the same source, read from the
  /// same two helpers rather than from a second copy of them — a chain of two or
  /// more static names whose base holds a group. Every path *up to* the last name
  /// is a prefix, because each of them is a group in its own right and the last
  /// name is the one that answers a variable.
  ///
  /// Cheapest questions first, and none of them costs a resolution the walk was
  /// not about to spend anyway: the chain's shape and its base's spelling are
  /// syntax, the scope is the guard's own, and the base is the very name the step
  /// below this resolves — through the evaluator's memo, so it is read once.
  fn record_a_dotted_theme_read(&mut self, member: &MemberExpr) {
    let Some((base, parts)) = get_full_member_path(member) else {
      return;
    };

    // The base of a chain that reads a group is the name the group is carried
    // under, which is the name the paths below belong to — and being a name is
    // also what makes a base able to hold a group at all. Both come off the one
    // reading the dispatch below asks of the same source.
    let Some(name) = theme_ref_base(&base) else {
      return;
    };

    // A name a callback binds is the callback's, whatever the module binds it to
    // — so a parameter shadowing a group reads properties off whatever it was
    // handed, and nothing about that chain belongs to the group.
    if self.guard.scope.binds(&name.sym) {
      return;
    }

    if !matches!(
      self.reader.resolve(&base),
      Some(EvaluateResultValue::ThemeRef(_))
    ) {
      return;
    }

    let paths = self
      .reader
      .dotted_prefixes
      .entry(name.sym.clone())
      .or_default();
    let mut path = String::new();

    for part in parts.iter().take(parts.len() - 1) {
      if !path.is_empty() {
        path.push('.');
      }

      path.push_str(part);

      paths.insert(Atom::from(path.as_str()));
    }
  }

  /// Whether `bin`'s right operand is evaluated at all.
  ///
  /// True for every operator that is not short-circuiting, and for one that is
  /// where its left operand does not settle the answer on its own. Which
  /// operands a short circuit reaches is the operator's own rule, read from
  /// [`evaluates_its_right_operand`] rather than restated here, so the walk and
  /// the fold cannot come to disagree about which side a build reaches.
  fn right_operand_runs(&mut self, bin: &BinExpr) -> bool {
    let Some(op) = LogicalOp::of(bin.op) else {
      return true;
    };

    match self.deciding_value_of(&bin.left) {
      Some(left) => evaluates_its_right_operand(op, &left),
      None => true,
    }
  }

  /// The arm of `test ? cons : alt` the engine evaluates and the arm it does
  /// not, and `None` where this walk cannot read which is which.
  ///
  /// Truthiness comes from the one bridge every other reader of it uses, so a
  /// test the conditional node would take the second arm for is the arm this walk
  /// carries. Both arms are answered rather than only the live one, because the
  /// dead one is still printed and the claim has to cover it.
  fn arms_of<'e>(&mut self, cond: &'e CondExpr) -> Option<(&'e Expr, &'e Expr)> {
    let test = self.deciding_value_of(&cond.test)?;

    match evaluate_result_to_js_boolean(&test)? {
      true => Some((&cond.cons, &cond.alt)),
      false => Some((&cond.alt, &cond.cons)),
    }
  }

  /// What `expr` holds, where a [dead
  /// operand](../../../../../CONTEXT.md#dead-operand) beside it may be decided
  /// from the module at all — and `None` where it may not, which both callers
  /// take for "walk both sides".
  ///
  /// Deciding is the correctness rather than a saving, and the glossary entry
  /// above is where that is argued. What is decided here is only *whether the
  /// module may be asked*: inside a callback it may not, because the engine binds
  /// the callback's own names, so a test written on one of them holds a different
  /// value per element and what the module resolves that spelling to is a
  /// different binding entirely — the pruned side's own names would then go
  /// unbound and the engine would reach for what nothing gave it. Asked of the
  /// scope rather than of the expression, because a name bound one arrow further
  /// out is as much the engine's as the parameter beside it.
  ///
  /// The operand this reads has already been walked by the caller, so it is read
  /// twice. Both readings go through the evaluator's memo, so the second is a
  /// hash of the subtree and a lookup rather than an evaluation, and it is paid
  /// only where a short-circuiting form was written.
  fn deciding_value_of(&mut self, expr: &Expr) -> Option<EvaluateResultValue> {
    match self.guard.scope.inside_a_callback() {
      true => None,
      false => self.reader.resolve(expr),
    }
  }

  /// Admits a name the module declared a function under, by crossing the
  /// declaration rather than a value.
  ///
  /// The declaration is walked like any other expression, so an arrow reached
  /// through a chain of names crosses as the chain — each link a parameter of the
  /// printed arrow, defaulted to what the link before it resolved. What the walk
  /// admits is therefore the same set of shapes an arrow written in place gets,
  /// and nothing about being named is asked separately.
  ///
  /// Reached only for a name the transport does not already carry, which the
  /// caller answers because it is what saves reading the declaration at all.
  fn admit_a_named_function(&mut self, ident: &Ident, declaration: Expr) -> Result<(), Decline> {
    // Walked in the scope the declaration was written in, which is the module. A
    // name the declaration reads is a module name however deep inside a callback
    // the reading of *this* name was, and the default it prints into stands in the
    // parameter list, where only the module names this fold carries are bound.
    //
    // Depth descends here rather than restarting, unlike the walk over a resolved
    // value: a declaration can name a second function whose declaration names the
    // first, and the descent is what bounds a walk that goes round.
    let outer = Guard {
      scope: Scope::Module,
      ..self.guard
    };

    self.under(outer).admit_value(&declaration)?;

    // Recorded after the walk, so every name the declaration reads is already a
    // parameter ahead of the parameter whose default reads it.
    self
      .reader
      .transport
      .carry(&ident.sym, Crossing::Source(Box::new(declaration)));

    Ok(())
  }

  /// The [StyleX function the engine may
  /// call](super::super::engine_stylex_functions) that `call` names, or `None`
  /// where it names none.
  ///
  /// A name the *callback* binds is not one of them however the module spelled its
  /// imports: the engine binds it when it runs the callback, and a value the guard
  /// carried under the same name would be shadowed by it anyway.
  fn a_stylex_function(&self, call: &CallExpr) -> Option<EngineCallable> {
    let Callee::Expr(callee) = &call.callee else {
      return None;
    };

    let callable = engine_callable(callee, self.reader.traversal_state)?;

    match self.guard.scope.binds(&callable.name) {
      true => None,
      false => Some(callable),
    }
  }

  /// Admits a call to a StyleX function: its arguments are values like any other,
  /// and the name it is reached through carries the function itself.
  ///
  /// An argument with no compile-time value is a rule *inside a callback* and a
  /// shape handed back outside one, which is the same question the [applied
  /// global](../../../../../CONTEXT.md#applied-global) answers by owning every one
  /// of its calls. Inside a callback nothing below the fold can answer: the array
  /// methods that would have run the body moved into the engine, so handing the
  /// call on ends it at a sentence about the *method*, naming neither the argument
  /// nor the reason. Outside one the dispatch may still own the call around this
  /// one, and a rule raised here would take a fold away from it.
  ///
  /// A call written on its own never reaches here at all: the fold walks a value,
  /// and the outermost call stays the dispatch's. That is deliberate — it resolves
  /// its arguments this compiler's own way, a theme reference included, and the
  /// engine holds no value for one of those.
  fn admit_a_stylex_function(
    &mut self,
    callable: &EngineCallable,
    call: &CallExpr,
  ) -> Result<(), Decline> {
    let inside_a_callback = self.guard.scope.inside_a_callback();

    self
      .admit_arguments(&call.args)
      .map_err(|declined| match declined {
        Decline::NotACandidate if inside_a_callback => {
          Decline::rule(uncoercible_value(callable.function_name()))
        },
        declined => declined,
      })?;

    // A name reached directly holds the function; a namespace holds an object of
    // the properties the engine may call. Both come from the callable itself, so
    // the value under a name is a function of the name and the transport's
    // one-value-per-name rule cannot drop a second naming.
    let crossing = match callable.reached {
      Reached::Directly => Crossing::Function(callable.call()),
      Reached::AsAProperty => Crossing::Namespace,
    };

    self.reader.transport.carry(&callable.name, crossing);

    Ok(())
  }

  /// Whether a call is one this module can hand to the engine whole.
  ///
  /// Every boundary is checked here rather than at the outermost call, because a
  /// chain hides its middle links: `"AB".toLocaleLowerCase().trim()` is a `trim`
  /// whose receiver needs a locale.
  ///
  /// What this function itself decides is which of three shapes the callee is —
  /// an applied global, a name, or a method — and each shape's own rules are the
  /// helper it hands the call to. Nothing is read off a value here, so a call this
  /// module was never going to fold costs the match and nothing else.
  ///
  /// `position` is read by one rule only — see [`Walk::admit_a_named_call`] — and
  /// is a parameter rather than something [`Guard`] carries, so nothing else on the
  /// walk can start depending on where it is.
  pub(super) fn admit_call<'c>(
    &mut self,
    call: &'c CallExpr,
    position: Position,
  ) -> Result<Admitted<'c>, Decline> {
    let Callee::Expr(callee) = &call.callee else {
      return Err(Decline::NotACandidate);
    };

    // Whatever the call around this one measured is not this call's business: only
    // the call an arrow is written directly inside says how often it runs. This
    // call sets it again below, for its own arguments.
    let guard = Guard {
      callback: None,
      ..self.guard
    };

    // `String(x)`, `Number(x)`, `Array(n)` and `Object(x)` are native JavaScript
    // functions, so they are folded by being called rather than by a conversion
    // written out here. A name the module bound is not one of them and is left to
    // the dispatch below, which calls the author's own function.
    if let Some(global) = unshadowed_applied_global(callee, self.reader.traversal_state) {
      return self.under(guard).admit_applied_global(global, call);
    }

    if let Some(name) = normalize_expr(callee).as_ident() {
      return self
        .under(guard)
        .admit_a_named_call(name, callee, call, position);
    }

    // Read through the callee's parentheses, as the two arms above read theirs.
    // The receiver keeps its own, because the fold prints this source back for
    // the engine to parse and `{ a: 1 }.valueOf()` is not an expression there.
    let Expr::Member(MemberExpr { obj, prop, .. }) = normalize_expr(callee) else {
      return Err(Decline::NotACandidate);
    };

    // A computed method name is a lookup the guard cannot resolve, even when it
    // is written as a literal.
    let MemberProp::Ident(method) = prop else {
      return Err(Decline::NotACandidate);
    };

    self.under(guard).admit_a_method_call(obj, method, call)
  }

  /// Whether a method call is one the engine can answer.
  ///
  /// The third of the three callee shapes [`Walk::admit_call`] dispatches on, and
  /// the one that carries the boundaries: the other two name a function, while a
  /// method has a receiver, and a receiver is where the rest is read.
  ///
  /// Nearly everything answerable from syntax is answered before the walk, because
  /// the walk resolves bindings and resolution is the only expensive thing here. So
  /// the spelling of the method name and a receiver written as a number are settled
  /// while they are still free, and only a call this module intends to fold pays to
  /// have its names read.
  ///
  /// Two rules sit behind the walk instead, and each is named where it is applied.
  /// The amplification bound is arithmetic on resolved values rather than a shape,
  /// so a call whose receiver the walk declines is not this fold's to price — it
  /// would otherwise report a ceiling for a receiver nothing had claimed, failing a
  /// build the dispatch below still answers. The escaping-property check is a name
  /// check that would cost nothing in front, and is behind so a chain of escaping
  /// reads is named for its outermost cause.
  ///
  /// `guard` arrives already applied by the caller, so the callback count this sets
  /// for its own arguments is the only one in scope.
  fn admit_a_method_call<'c>(
    &mut self,
    obj: &Expr,
    method: &'c IdentName,
    call: &CallExpr,
  ) -> Result<Admitted<'c>, Decline> {
    let guard = self.guard;

    // A receiver naming one of the globals the engine provides itself — `Math`,
    // `Object`, `String`, `Number`, `Array` — carries no value across the bridge:
    // the printed source names it and the language answers. That is the whole of
    // what folding a static needs, so the surface is the language's rather than a
    // list of names this compiler chose, and where a static is written no longer
    // decides whether it folds.
    let global = unshadowed_receiver_global(obj, self.reader.traversal_state);

    refuse_a_method_by_its_spelling(obj, method, global)?;

    // Two things on the receiver, and both skipped for a global.
    //
    // A global the engine provides itself carries no value across the bridge: the
    // printed source names it and the language answers. It is admitted as a
    // receiver here and nowhere else — a global's *name* is not a value this fold
    // carries, and admitting it as one would let `['a'].concat(String)` fold a
    // function's own source text into a declaration.
    //
    // Then the count a callback among the arguments would repeat, read off the same
    // receiver — after it has been admitted, so a receiver the guard is about to
    // refuse is never read for a count nothing will use. A global holds no elements
    // of its own, so the only static counted there is the one that iterates an
    // argument.
    let counted = match global {
      None => {
        self.under(guard).admit_value(obj)?;

        self
          .under(guard)
          .admitted_callback(&method.sym, obj, &call.args)
      },
      // `Array.from` is the one static that runs a callback once per element,
      // and what it iterates is its first argument rather than a receiver.
      Some(global) => match EntryAmplifier::named(global, Some(&method.sym)) {
        Some(EntryAmplifier::From) => self.under(guard).admitted_mapper(&call.args),
        _ => None,
      },
    };

    // The allocation bounds, and they run *after* the receiver above because they
    // are arithmetic on values rather than on syntax: a count or a receiver may be
    // written out, and may equally be a name they resolve. A call whose receiver
    // this fold cannot claim is not this fold's to price — reporting a ceiling for
    // it would fail a build the dispatch below still answers, and the dispatch is
    // what a decline hands the call back to.
    //
    // The cost of that ordering is a walk of the receiver on a call these will
    // refuse anyway, which is a build that fails either way and pays one resolved
    // binding for the better sentence.
    self
      .under(guard)
      .admit_amplification(&method.sym, obj, &call.args)?;

    // `Array.from` is the other spelling of a declared length, and the only static
    // that carries one: every remaining `Array` static answers a length its own
    // arguments write out.
    if let Some(global) = global
      && let Some(amplifier) = EntryAmplifier::named(global, Some(&method.sym))
    {
      self
        .under(guard)
        .admit_entry_amplification(amplifier, &call.args)?;
    }

    let inner = Guard {
      callback: counted,
      ..guard
    };

    self.under(inner).admit_arguments(&call.args)?;

    // The one rule left behind the walk, and deliberately. It is a name check like
    // the three above and would cost nothing in front of them, but a chain of
    // escaping reads is refused outermost-first there — so
    // `''.constructor.constructor('return 1').call()` would be named for its
    // `call` rather than for the `constructor` that is the whole of the reason.
    // The walk reaches the receiver's reads first, which is the sentence worth
    // reading, and the resolution it costs is one binding on a call already
    // certain to refuse.

    // A call spelled `x.__proto__()` is not a call any value answers, so the
    // second of the two rules only decides which sentence the refusal reads --
    // and the rule's sentence says more than the engine's "not a callable
    // function" would.
    if let Some(refusal) = refusal_for_a_property_name(&method.sym) {
      return Err(Decline::Rule(refusal));
    }

    Ok(Admitted::method(&method.sym))
  }

  /// Whether a call applying a global is one the engine can answer.
  ///
  /// The arguments are walked as values and nothing else: none of the globals is a
  /// higher-order function, so an arrow among them is a value like any other.
  ///
  /// An argument the bridge cannot carry hands the call back rather than refusing
  /// it. The bridge carries JavaScript values, and this compiler has values of its
  /// own — the injected function map and the environment object — which have no
  /// JavaScript form to cross as, so the engine cannot be the thing that answers
  /// for them. The dispatch below folds a global applied to
  /// one, and raises [`uncoercible_value`] where it cannot, so the sentence a
  /// refusal carries is still written in one place.
  ///
  /// A rule the walk *did* name is still a refusal: it is about the argument the
  /// author wrote rather than about what the bridge holds, and nothing below the
  /// fold would name it again.
  ///
  /// The one thing the guard does not answer is whether the global is a function
  /// at all: that is the language's answer and is read off the engine in
  /// [`admit_an_applied_global`].
  fn admit_applied_global<'c>(
    &mut self,
    global: &'c Atom,
    call: &CallExpr,
  ) -> Result<Admitted<'c>, Decline> {
    if let Some(amplifier) = EntryAmplifier::named(global, None) {
      self.admit_entry_amplification(amplifier, &call.args)?;
    }

    self.admit_arguments(&call.args)?;

    Ok(Admitted::global(global))
  }

  /// Whether a call whose callee is a bare name is one the engine can answer.
  ///
  /// The callee is walked as a value like any other, so nothing about reaching the
  /// function is asked here: a name the surrounding callback binds is the engine's
  /// own and carries nothing, and a name the module bound to a function crosses as
  /// the declaration it came from — the same carriage a callback *passed* by name
  /// takes. What this adds is the dispatch, and the rule it applies is where the
  /// call sits.
  ///
  /// **The outermost call stays the dispatch's**, which is the line
  /// [`Walk::admit_a_stylex_function`] already draws and for the same reason: below
  /// the fold a call through a name is resolved this compiler's own way — a dynamic
  /// style's own parameters and the injected function map are answered there, and
  /// the engine holds no value for either of them. Measured, that path already
  /// folds `inner('a')` to upstream's rule, so
  /// taking the call would replace a working answer with a narrower one.
  ///
  /// A call *inside* an expression the fold claimed has no such second answer.
  /// Handing one back hands back the whole expression around it, and the method
  /// that would have re-run the body moved into the engine — so this is the only
  /// place that can answer, and the reason is the one the outermost call gives for
  /// not being answered here.
  fn admit_a_named_call<'c>(
    &mut self,
    name: &'c Ident,
    callee: &Expr,
    call: &CallExpr,
    position: Position,
  ) -> Result<Admitted<'c>, Decline> {
    if position == Position::Outermost {
      return Err(Decline::NotACandidate);
    }

    // The callee is an arrow this call runs once per evaluation of itself, rather
    // than once per element of a receiver — so what its body repeats is what the
    // expression around it repeats, and a length written into that body is bounded
    // like any other instead of refusing as a callback over something unmeasured.
    // No width travels with the count: a parameter here holds an argument, and an
    // argument's width is not something this reading measured.
    let callee_guard = Guard {
      callback: Some(Callback {
        repeats: self.guard.repeats,
        ..Callback::default()
      }),
      ..self.guard
    };

    self.under(callee_guard).admit_value(callee)?;

    // The arguments keep the guard's own `callback`, which the call around this one
    // already cleared: an arrow handed to the author's own function is run by a
    // body this fold cannot see, so how often it runs is exactly what nothing here
    // measured.
    self.admit_arguments(&call.args)?;

    Ok(Admitted::named(&name.sym))
  }

  /// An argument is admitted when it is a value the walk carries — an arrow among
  /// them, which is how a callback and an own conversion method reach the engine.
  /// Every argument of a call, walked as a value.
  ///
  /// One loop rather than one per calling shape, so a position that maps the
  /// refusal — an applied global's, a StyleX function's — differs from the others
  /// in the mapping alone and not in what it walked.
  fn admit_arguments(&mut self, args: &[ExprOrSpread]) -> Result<(), Decline> {
    for arg in args {
      self.admit_argument(arg)?;
    }

    Ok(())
  }

  fn admit_argument(&mut self, arg: &ExprOrSpread) -> Result<(), Decline> {
    // A spread needs the scope, and is refused rather than handed back: the
    // receiver is walked before the arguments, so a call reaching here is one this
    // module owns, and the sentence for a spread is the same one every other
    // position gives it.
    if arg.spread.is_some() {
      return Err(Decline::rule(SPREAD_ELEMENT));
    }

    self.admit_value(&arg.expr)
  }

  /// Whether an arrow reads nothing but the names it binds and names the module
  /// resolves. Anything else would need a scope the engine does not have.
  ///
  /// The arrow itself is not analysed — the engine parses it, so a destructured
  /// parameter and a block body are shapes the language answers rather than
  /// shapes this guard has to recognise. What the walk still does is name what
  /// the arrow binds, so a read of one is not asked of the module, and apply to
  /// the body the rules every other position gets: a callback body is source that
  /// really runs.
  fn admit_arrow(&mut self, arrow: &ArrowExpr) -> Result<(), Decline> {
    let mut bindings = Bindings::default();
    let mut spans = Vec::with_capacity(arrow.params.len());

    for param in &arrow.params {
      // Where each parameter's names begin and end, so a value the call measured
      // is bound to the names of the parameter it is really handed to rather
      // than to whichever came first.
      let from = bindings.names.len();

      bindings.pattern(param, self.guard.depth)?;

      spans.push((from, bindings.names.len()));
    }

    // What the call this arrow was written inside measured for it. A parameter with
    // a default may be handed something else entirely, so a defaulted parameter
    // list takes neither the width nor the count.
    let measured = match bindings.evaluates.is_empty() {
      true => self.guard.callback,
      false => None,
    };

    let (elements, repeats) = match measured {
      Some(callback) => (
        Elements::placed(callback, &arrow.params, &spans),
        callback.repeats,
      ),
      None => (Elements::default(), Repeats::Unmeasured),
    };

    // Copied out of the walk because the scope the names are bound over is
    // borrowed from it, and the sub-walk below borrows the reader beside it.
    let outer = self.guard;
    let inner = bindings.enter(&outer, elements, repeats, self.reader)?;

    match arrow.body.as_ref() {
      ArrowFunctionBody::Expr(body) => self.under(inner).admit_value(body),
      ArrowFunctionBody::FunctionBody(body) => self.under(inner).admit_block(&body.stmts),
    }
  }

  /// A block, admitted as a scope of its own: what it declares is bound before
  /// its statements are walked, so a declaration reads as itself rather than as a
  /// module name it shadows.
  ///
  /// A `var` is function-scoped and is bound here as though it were not. That
  /// only narrows what is visible — a read of one from an enclosing block finds
  /// no binding and the call is handed back — so it costs a fold rather than
  /// answering one wrongly.
  fn admit_block(&mut self, stmts: &[Stmt]) -> Result<(), Decline> {
    let outer = self.guard.descend()?;
    let mut bindings = Bindings::default();

    for stmt in stmts {
      if let Stmt::Decl(Decl::Var(declaration)) = stmt {
        for declarator in &declaration.decls {
          bindings.pattern(&declarator.name, outer.depth)?;
        }
      }
    }

    // A block declares values its own body built, so nothing here is an element of
    // a measured receiver — and the repeats are the ones the body around it is
    // already counted at.
    let inner = bindings.enter(&outer, Elements::default(), outer.repeats, self.reader)?;

    for stmt in stmts {
      self.under(inner).admit_statement(stmt)?;
    }

    Ok(())
  }

  /// A statement inside a callback body.
  ///
  /// The set is the statements that compute a value and hand it back, which is
  /// all a callback is for. What is left out is left out for a reason, and each
  /// one is written here.
  ///
  /// A **loop** is bounded by the engine
  /// ([`MAX_LOOP_ITERATIONS`](super::engine::MAX_LOOP_ITERATIONS)), but the count
  /// that bound is applied to lives on the *call frame* — so a callback invoked
  /// once per element starts a fresh count every time, and the bound is multiplied
  /// by an element count the source never states. That is the same arithmetic
  /// [`Walk::admit_amplification`] refuses inside a callback, and every loop this
  /// walk can reach is inside one, since a statement is only ever walked in a
  /// callback body.
  ///
  /// A **function or class declaration** carries a body this walk does not read,
  /// and neither does the value walk read one written as an expression.
  ///
  /// Both are refusals and not "not mine". The exclusion is a boundary this
  /// module owns with a reason written down, and a boundary like that has to
  /// answer in this module's words — handing the shape back instead ends it at
  /// the dispatch's `Unsupported expression: ArrowFunctionExpression`, which
  /// names the callback rather than the statement inside it and leaves an author
  /// to guess.
  ///
  /// An assignment is not among them, because it is an expression rather than a
  /// statement: `v = 1;` is a `Stmt::Expr`, so it is answered by the value walk,
  /// which does not model it and hands the call back like every other expression
  /// kind it does not read.
  fn admit_statement(&mut self, stmt: &Stmt) -> Result<(), Decline> {
    grown_per_level(|| self.nested_statement(stmt))
  }

  /// One statement, on the room [`admit_statement`](Walk::admit_statement) asked
  /// for, and reached only through it — a direct call would descend on no room at
  /// all.
  fn nested_statement(&mut self, stmt: &Stmt) -> Result<(), Decline> {
    let inner = self.guard.descend()?;

    match stmt {
      Stmt::Expr(ExprStmt { expr, .. }) => self.under(inner).admit_value(expr),
      Stmt::Return(ReturnStmt { arg, .. }) => match arg {
        Some(value) => self.under(inner).admit_value(value),
        None => Ok(()),
      },
      // The names were bound by the block around this, so only the initialisers
      // are walked here.
      Stmt::Decl(Decl::Var(declaration)) => {
        for declarator in &declaration.decls {
          if let Some(init) = &declarator.init {
            self.under(inner).admit_value(init)?;
          }
        }

        Ok(())
      },
      Stmt::Block(block) => self.under(inner).admit_block(&block.stmts),
      Stmt::If(branch) => {
        self.under(inner).admit_value(&branch.test)?;
        self.under(inner).admit_statement(&branch.cons)?;

        match &branch.alt {
          Some(alt) => self.under(inner).admit_statement(alt),
          None => Ok(()),
        }
      },
      // A stray semicolon binds nothing and evaluates nothing.
      Stmt::Empty(_) => Ok(()),
      _ => Err(Decline::rule(unfoldable_statement(get_stmt_node_kind(
        stmt,
      )))),
    }
  }
}

/// Refuses an applied global that is not a function.
///
/// Asked of the language rather than of a list of names: the global object holds
/// the value, and the value says whether it can be applied. The engine's own
/// sentence for applying one is `not a callable function`, which names neither
/// the global nor the mistake, so the refusal is this compiler's.
///
/// Asked with the engine in hand and so after the guard, because it is the one
/// rule here that cannot be answered from the source. Asked of the outermost call
/// only: a global applied in the middle of a chain reads the engine's own throw,
/// and there is no sentence about a chain worth preferring to it.
pub(super) fn admit_an_applied_global(name: &Atom, engine: &mut Context) -> Result<(), Decline> {
  let global_object = engine.global_object();
  let value = read(name, || {
    global_object.get(JsString::from(name.as_str()), engine)
  })?;

  match value.is_callable() {
    true => Ok(()),
    false => Err(Decline::rule(not_a_function(name))),
  }
}

/// The identifier a global is written as, through parentheses, before any
/// question of shadowing.
///
/// Read through parentheses, because they change nothing about which name is
/// written and the reference compiler folds `(String)('a')` and `(Math).max(1, 2)`
/// exactly as it folds them bare.
fn written_global(expr: &Expr) -> Option<&Ident> {
  let expr = normalize_expr(expr);

  match expr.as_ident() {
    Some(ident) if is_valid_callee(expr) => Some(ident),
    _ => None,
  }
}

/// The name an applied global names, or `None` where the module bound it.
///
/// **Every** binding shadows here — a `const`, a hoisted `function`, a `class`,
/// an import — because folding a call the module owns is the one direction that
/// invents output: this compiler would name a class the other build never
/// defines, hashed from a declaration it never wrote. Measured, the reference
/// compiler refuses each of those shapes for the declaration it names, and a
/// name this returns `None` for reaches exactly that chain below the fold.
///
/// A binding is not the same as a refusal. `const String = (x) => 'no'` is a
/// function this compiler can call, and the dispatch below calls it — which is
/// what the reference compiler does too.
///
/// One `Id` probe and no evaluation, so it stays in front of the walk with the
/// other cheap answers.
pub(in super::super) fn unshadowed_applied_global<'a>(
  expr: &'a Expr,
  state: &StateManager,
) -> Option<&'a Atom> {
  let ident = written_global(expr)?;

  // Read in the same order as the receiver rule below -- the global first, the
  // shadow second -- so the difference a reader is looking for is the question
  // each asks and not the shape of the answer.
  match state.declares_binding(ident) {
    false => Some(&ident.sym),
    true => None,
  }
}

/// The name a static's receiver names as a global, or `None` where the module
/// declared a value under it.
///
/// **Narrower than the callee rule above, and the two go opposite ways on
/// purpose.** A receiver carries no value across the bridge: the printed source
/// names it and the language answers, so a `function` or a `class` of the same
/// name changes nothing about the static that folds — and the reference compiler
/// folds `Math.max(1, 2)` under `function Math() {}` for exactly that reason.
/// Only a declarator is read here, because only a declarator holds a value the
/// static could have been meant to read.
///
/// A declarator that does hold one is the module's own value and is resolved
/// like any other name: measured, `const String = 'abc'; String.toUpperCase()`
/// folds to `ABC` in the reference compiler, so treating the name as the global
/// would refuse an input it compiles.
///
/// Where the value it holds is an *object*, the two compilers part: `const Math
/// = { trunc: () => 9 }; Math.trunc(1.5)` is `1` upstream, which reads the
/// shadow's name and the global's method and so answers for neither. Refusing is
/// the safe direction — a refusal leaves the call to the runtime where a wrong
/// fold writes a wrong declaration — and it is the direction the callee rule
/// cannot take, since only a callee fold can name a class the other build never
/// defines. See `ADR 0008`.
fn unshadowed_receiver_global<'a>(expr: &'a Expr, state: &StateManager) -> Option<&'a Atom> {
  let ident = written_global(expr)?;

  match state.declaration_of(ident) {
    None => Some(&ident.sym),
    Some(_) => None,
  }
}

/// Whether a name is one of the globals the fold recognises, standing where a
/// value belongs rather than being called or read from.
///
/// The callees the fold already owns, read through the same set membership the
/// callee rule reads, plus the ones it recognises without ever calling — see
/// [`VALUE_ONLY_GLOBALS`].
///
/// Every binding shadows, exactly as it does for a callee: a module that bound
/// the spelling owns it, whatever it bound it to, and the rules above answer for
/// what it holds.
///
/// Asked of the name rather than of the expression around it, unlike the two
/// rules above: the walk unwraps a parenthesis before it dispatches, so the one
/// caller already holds the name it would have read.
fn a_global_written_as_a_value(ident: &Ident, state: &StateManager) -> bool {
  let named = is_a_valid_callee_name(&ident.sym) || VALUE_ONLY_GLOBALS.contains(ident.sym.as_ref());

  named && !state.declares_binding(ident)
}

/// The expression a name was declared with, or `None` where the module declares
/// no such binding.
///
/// One walk from a name to its initializer, because both questions below start
/// with it: what a function crosses as, and whether a name the guard could not
/// resolve was a function at all. Two spellings of the same three links would
/// have been two chances to disagree about which link is optional.
///
/// Asked of the name rather than of the expression around it, for the reason
/// [`a_global_written_as_a_value`] is: the walk unwraps a parenthesis before it
/// dispatches, so every caller already holds the name it would have read.
fn initializer_of<'a>(ident: &Ident, reader: &'a Reader) -> Option<&'a Expr> {
  reader
    .traversal_state
    .declaration_of(ident)
    .and_then(|declarator| declarator.init.as_deref())
}

/// Whether the module declares `ident` as a function, which is what makes a name
/// the guard could not resolve a refusal rather than a call to hand back.
///
/// Three declarations answer yes here, and they are the three the reference
/// compiler refuses too: an arrow the transport could not take, because a block
/// body or a destructured parameter is a shape the evaluator answers no callback
/// for; a `function` of either spelling; and a binding written to after it was
/// declared, which is refused for the write rather than for its shape.
///
/// A `function` declaration is asked of the declaration list rather than of a
/// declarator, because it is hoisted and has no initializer to read.
fn the_module_declares_a_function(ident: &Ident, reader: &Reader) -> bool {
  matches!(
    reader.traversal_state.declared_as(ident),
    Some(DeclarationType::Function)
  ) || initializer_of(ident, reader)
    .is_some_and(|init| matches!(init, Expr::Arrow(_) | Expr::Fn(_)))
}

/// The refusals a method call can answer from its own text.
///
/// First of the three phases [`Walk::admit_a_method_call`] runs, and the only one
/// that resolves nothing: each rule reads the method's spelling or the shape the
/// receiver was written as, so none of them can refuse a call on the strength of
/// a binding the walk went and looked up. That is what makes running them first
/// free, and it is why they are out here as a function rather than a method --
/// nothing in the walk is in reach, so nothing in the walk can be leaned on.
///
/// `global` is the receiver's own name where it is one of the language's, which
/// the first rule needs and the other two do not.
fn refuse_a_method_by_its_spelling(
  obj: &Expr,
  method: &IdentName,
  global: Option<&Atom>,
) -> Result<(), Decline> {
  // A static is folded only where the global's own allowlist holds it. An
  // allowlist and not a denylist, because `Object` carries reflective statics
  // that hand back an object from the prototype chain, and a denylist over that
  // surface can never be complete -- the static nobody listed is the next way
  // out. The rest of the surface is refused for the reason it always was: a
  // fold has to be a function of the source, and a static that mutates what it
  // was handed, or answers anew on every build, is not.
  //
  // Two sentences, because the two halves are refused for different reasons and
  // an author acts on each differently.
  if let Some(global) = global
    && !is_valid_callee_method_name(global, &method.sym)
  {
    // A global's `constructor` is the shortest escape there is, so it reads as
    // one rather than as a static that could not be folded. Every other name
    // outside the allowlist reads the one sentence written for them.
    let refusal = refusal_for_a_property_name(&method.sym)
      .unwrap_or_else(|| unfoldable_static(global, &method.sym).into());

    return Err(Decline::Rule(refusal));
  }

  // A method whose answer needs locale data the engine does not carry.
  if lists(&LOCALE_SENSITIVE_METHODS, &method.sym) {
    return Err(Decline::rule(locale_sensitive_method(&method.sym)));
  }

  // A receiver written as a number, which the reference compiler applies the
  // method without, so refusing keeps both compilers rejecting one input.
  if receiver_is_a_written_number(obj) {
    return Err(Decline::rule(numeric_literal_receiver(&method.sym)));
  }

  Ok(())
}

/// Whether the receiver is a number written into the source as a literal.
///
/// Every `Number.prototype` method throws upstream on one of those — the
/// reference implementation applies the method without a receiver, so
/// `(5).toFixed(2)` reports that `toFixed` requires a Number. Refusing keeps
/// both compilers rejecting the same input. A number that a fold *produced* is
/// a different shape and folds in both, as does a negated literal, which is a
/// unary expression rather than a literal.
fn receiver_is_a_written_number(receiver: &Expr) -> bool {
  match receiver {
    Expr::Paren(paren) => receiver_is_a_written_number(&paren.expr),
    Expr::Lit(lit) => matches!(lit, Lit::Num(_)),
    _ => false,
  }
}

/// Whether a *name* may hold this value — a question of its own, and narrower
/// than what the bridge [carries](super::transport::Crossing::Value). Narrower,
/// too, than the set of receivers the dispatch below hands straight back to a
/// refusal: that one is about which prototypes this module now owns whole, and
/// an object is not among them because an object receiver is still where a
/// function map's own methods are looked up.
///
/// A string, an array, a plain object, a number and a boolean — the primitives
/// and the two composites the bridge was proved on, in the one shape that
/// matters to the guard: a value the engine can be handed and the source can go
/// on reading as itself.
///
/// A number is here because the statics need it. `Math.round(BASE / 4)` is the
/// ordinary way to write one, and `BASE` is a name; refusing a name that holds a
/// number would have refused every arithmetic fold whose operands were not
/// written out. It follows that `const n = 255; n.toString(16)` folds too, which
/// is what the reference compiler does. The refusal that has to survive is about
/// how the receiver was *written*, not what it holds — a number literal in the
/// source is still refused, in [`receiver_is_a_written_number`], because the
/// reference compiler throws on one.
///
/// A boolean comes with it rather than for a case of its own. Once a name may
/// hold a number, a boolean is the only primitive left outside, and there is no
/// sentence that would say why: it prints, it crosses, and its prototype folds
/// like any other. Leaving it out would be a table of one — the shape this whole
/// module exists to delete.
fn is_a_carryable_receiver(value: &EvaluateResultValue) -> bool {
  matches!(
    value,
    EvaluateResultValue::Vec(_)
      | EvaluateResultValue::ThemeRef(_)
      | EvaluateResultValue::Expr(
        Expr::Lit(Lit::Str(_) | Lit::Num(_) | Lit::Bool(_)) | Expr::Array(_) | Expr::Object(_)
      )
  )
}

#[cfg(test)]
#[path = "tests/speculation_tests.rs"]
mod speculation_tests;

#[cfg(test)]
#[path = "tests/shadowed_names_tests.rs"]
mod shadowed_names_tests;
