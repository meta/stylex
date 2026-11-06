//! Owner-scoped promotion of source heap allocations. Never hold the ledger
//! borrow while running getters, host callbacks or other re-entrant operations.
use super::*;
use std::{
  any::Any,
  cell::RefCell,
  rc::{Rc, Weak},
};
use stylex_structures::stylex_env::{HostRef, HostValue};
use stylex_types::enums::data_structures::injectable_style::InjectableStyleKind;
use stylex_types::structures::style_key::RuleKey;
use swc_core::ecma::ast::{MemberProp, Prop, PropName};

#[derive(Clone)]
struct Promotion {
  allocation: Weak<dyn Any>,
  reference: HostRef,
}
#[derive(Default)]
pub struct EvaluationHeap {
  promotions: FxHashMap<(u64, u8, usize), Promotion>,
  functions: FxHashMap<(u64, usize), (Weak<dyn Any>, HostValue)>,
  /// Effects an immutable host operation produced, waiting for a writer that
  /// holds the state mutably. Appended here rather than threaded through the
  /// read paths; see [`StateManager::queue_evaluation_effects`]. Default, so no
  /// existing initializer of this struct changes.
  pending: Vec<EvaluationEffect>,
  /// Whether the queue refused an effect for want of room, which is terminal.
  pending_overflowed: bool,
}
impl std::fmt::Debug for EvaluationHeap {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("EvaluationHeap")
      .field("promotions", &self.promotions.len())
      .finish()
  }
}
impl EvaluationHeap {
  pub fn function(&self, owner: u64, allocation: &Rc<dyn Any>) -> Option<HostValue> {
    let (witness, value) = self
      .functions
      .get(&(owner, Rc::as_ptr(allocation) as *const () as usize))?;
    let alive = witness.upgrade()?;
    Rc::ptr_eq(&alive, allocation).then(|| value.clone())
  }
  pub fn record_function(&mut self, owner: u64, allocation: &Rc<dyn Any>, value: HostValue) {
    self.functions.insert(
      (owner, Rc::as_ptr(allocation) as *const () as usize),
      (Rc::downgrade(allocation), value),
    );
  }

  pub fn reference(&self, owner: u64, kind: u8, allocation: &Rc<dyn Any>) -> Option<HostRef> {
    let key = (owner, kind, Rc::as_ptr(allocation) as *const () as usize);
    let promoted = self.promotions.get(&key)?;
    let alive = promoted.allocation.upgrade()?;
    Rc::ptr_eq(&alive, allocation).then(|| promoted.reference.clone())
  }
  pub fn record(&mut self, owner: u64, kind: u8, allocation: &Rc<dyn Any>, reference: HostRef) {
    let key = (owner, kind, Rc::as_ptr(allocation) as *const () as usize);
    self.promotions.insert(
      key,
      Promotion {
        allocation: Rc::downgrade(allocation),
        reference,
      },
    );
  }
}
impl StateManager {
  pub fn evaluation_heap(&self) -> Rc<RefCell<EvaluationHeap>> {
    self.evaluation_heap.clone()
  }
}

// ==================== a selective snapshot of the module's state ====================

/// The floor on how many distinct bindings one snapshot carries.
///
/// A floor rather than a ceiling: the real bound is scaled from the module's
/// own size by [`snapshot_bounds`], so a large file admits a callback that
/// reaches further, and this is only the smallest answer that number can take.
/// It is sized so a hand-written module never meets a bound at all.
pub const MIN_SNAPSHOT_BINDINGS: usize = 256;

/// The floor on how many identifier references one initializer contributes.
///
/// Same reasoning one level down, and the same scaling.
pub const MIN_REFERENCES_PER_INITIALIZER: usize = 4_096;

/// How much of the scaled fuel one carried binding is charged.
///
/// A binding costs a hash probe against two indexes and a bounded walk of one
/// initializer, so it is the expensive unit; a reference costs a push. The two
/// divisors are what tie the bounds below to the same input-sized budget the
/// evaluation itself runs under, rather than to a number chosen here.
const FUEL_PER_BINDING: usize = 256;

/// How much of the scaled fuel one followed reference is charged.
const FUEL_PER_REFERENCE: usize = 16;

/// What one snapshot of a module this size may walk.
///
/// Derived from the same input-sized budget `stylex_js::native` grants an
/// evaluation of the same module -- its minimum fuel, raised in proportion to
/// the module's bytes -- so a callback the shared budget would let run is not
/// refused here for a number this module invented. The floors keep a small
/// module's bound at the hand-written size rather than at whatever the minimum
/// fuel happens to divide to.
#[derive(Clone, Copy, Debug)]
pub struct SnapshotBounds {
  pub bindings: usize,
  pub references: usize,
}

/// The bounds a module of `bytes` admits.
pub fn snapshot_bounds(bytes: usize) -> SnapshotBounds {
  // The native budget's own scaling, read in the shape `Budget::scale_input`
  // writes it: the minimum fuel, or four units per input byte where that is
  // larger.
  let fuel = stylex_js::native::MAX_FUEL.max(bytes.saturating_mul(4));

  SnapshotBounds {
    bindings: (fuel / FUEL_PER_BINDING).max(MIN_SNAPSHOT_BINDINGS),
    references: (fuel / FUEL_PER_REFERENCE).max(MIN_REFERENCES_PER_INITIALIZER),
  }
}

/// The part of a module's state a retained callback can still read.
///
/// A callback an owner holds past the expression that built it may be invoked
/// during a later transform, where the state it closed over is gone. Cloning
/// the whole `StateManager` to keep it alive is what this replaces: that copies
/// the compiled style map, the injected rules and the evaluator memo, none of
/// which a callback body reads, and all of which are the parts that scale with
/// the size of the file the callback happens to sit in.
///
/// What is carried instead is what resolving a name needs: the declarations the
/// callback can reach, the imports that bind them, the four safety records a
/// reference is checked against, the options, the host, and the parsed source.
/// Everything else starts empty, so anything the callback records is its own --
/// which is what [`EvaluationEffects::harvest`] relies on.
///
/// The heap is **shared**, not copied: a promotion made before the snapshot is
/// still a promotion after it, so a source object that already crossed into the
/// host keeps the identity it crossed with.
///
/// A snapshot whose walk hit its bound is **terminal**: [`Self::spawn`] answers
/// nothing for one. A partial snapshot would fold a name to one value where the
/// module folds it to another -- the dependency it did not reach is missing
/// rather than absent -- and answering with it is the one outcome worse than
/// refusing.
pub struct EvaluationSnapshot {
  state: StateManager,
  reached: FxHashSet<Id>,
  /// The names the walk had left to follow when it hit its bound. Empty for
  /// every snapshot that completed.
  omitted: Vec<Id>,
  bounds: SnapshotBounds,
}

impl std::fmt::Debug for EvaluationSnapshot {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("EvaluationSnapshot")
      .field("bindings", &self.state.declarations().len())
      .field("imports", &self.state.top_imports.len())
      .field("reached", &self.reached.len())
      .field("omitted", &self.omitted.len())
      .field("bounds", &self.bounds)
      .finish()
  }
}

impl EvaluationSnapshot {
  /// A state to run one invocation against, or nothing where the walk was cut.
  ///
  /// Cloned per invocation rather than shared, because the evaluator writes to
  /// the state it is handed -- a memo entry, a depth counter, a filed rule --
  /// and two invocations must not see each other's. The clone is of the small
  /// state built above, so what it costs is the reached bindings and a handful
  /// of `Rc` bumps rather than the module.
  ///
  /// `None` is a truncated walk, and it is `None` rather than a partial state
  /// on purpose: see the type's own note. The caller reports it with the names
  /// [`Self::omitted`] holds.
  pub fn spawn(&self) -> Option<StateManager> {
    (!self.is_truncated()).then(|| self.state.clone())
  }

  /// Seeds the call shapes a body is about to be folded with.
  ///
  /// A member read that is the callee of a call is left to the call rather than
  /// evaluated as a read, and what decides that is whether the state has seen
  /// the call -- `StateManager::is_member_call_callee`. The module's own record
  /// of every call it holds is not carried, because it is a map the size of the
  /// file; the calls *this body* writes are, and they are the only ones the
  /// question can be asked about while this body folds.
  ///
  /// Call it with the body the callback will fold, before the first
  /// [`Self::spawn`]. A snapshot nobody seeds answers `false` for every such
  /// question, which reads a member callee as a plain member -- so this is not
  /// optional for a body that writes `stylex.create(...)` or any other call on
  /// a member.
  ///
  /// Bounded by the body's own nesting, through the same ceiling the reference
  /// walk uses.
  pub fn observe_calls_in(&mut self, body: &Expr) {
    let ceiling = self.state.evaluation_ceiling();
    let mut walk = SeenCalls {
      state: &mut self.state,
      depth: 0,
      ceiling,
    };

    body.visit_with(&mut walk);
  }

  /// How many names the walk resolved to something it carried.
  pub fn reached(&self) -> &FxHashSet<Id> {
    &self.reached
  }

  /// The dependencies the walk did not follow, which is empty unless the bound
  /// was hit. A caller reports these; it may not fold against them.
  pub fn omitted(&self) -> &[Id] {
    &self.omitted
  }

  /// Whether the walk stopped at its bound with names left to follow.
  pub fn is_truncated(&self) -> bool {
    !self.omitted.is_empty()
  }

  /// The bounds this snapshot was walked under, scaled from the module's size.
  pub fn bounds(&self) -> SnapshotBounds {
    self.bounds
  }

  /// The declarations the snapshot carries, in the order the walk reached them.
  pub fn declarations(&self) -> &[VarDeclarator] {
    self.state.declarations()
  }

  /// How many hoisted `class` and `function` records travelled.
  ///
  /// The module holds one per such declaration in the file; a snapshot holds
  /// one per *reached* name. Exposed so the narrowing is a number a case can
  /// assert rather than a claim in a comment -- a thousand unrelated helpers
  /// must add nothing here.
  pub fn declaration_records(&self) -> usize {
    self.state.declarations_state.class_names.len()
      + self.state.declarations_state.function_names.len()
      + self.state.declarations_state.opaque_faults.len()
  }

  /// How many import paths travelled, for the same reason.
  pub fn import_paths(&self) -> usize {
    self.state.imports.import_paths.len()
  }

  /// Whether the snapshot retained the import path `source`.
  ///
  /// The module's own record of which files it imports, narrowed to what this
  /// snapshot carries. Not to be confused with
  /// [`StateManager::is_import_source`], which answers a different question
  /// entirely -- whether a path is one of the *configured* StyleX import
  /// sources, read off the options and true of no ordinary module import.
  ///
  /// Exposed because a count alone cannot say that the right two paths
  /// survived: a narrowing that kept two wrong ones would pass a count and
  /// fail here.
  pub fn retains_import_path(&self, source: &str) -> bool {
    self.state.imports.import_paths.contains(source)
  }
}

impl StateManager {
  /// The state a callback that reads `ids` needs, and nothing else.
  ///
  /// `ids` are the free names of the body the callback will fold -- the
  /// evaluator above knows them, this crate does not. Every one is resolved
  /// against this module the way a reference is resolved during a fold, through
  /// [`Self::declaration_of`] and [`Self::import_binding`], so the snapshot
  /// cannot come to disagree with the resolver about what a name means. The
  /// identifiers the walk builds carry the original `Id` -- symbol *and*
  /// `SyntaxContext` -- so a shadowing binding resolves to itself here as it
  /// does there.
  ///
  /// A carried declarator's own initializer is walked for the names *it* reads,
  /// transitively. The walk is bounded by [`snapshot_bounds`], which scales
  /// both bounds from the module's own byte count against the same minimum
  /// fuel the native budget grants an evaluation of it -- so a callback the
  /// shared budget would admit is never refused by a number chosen here. A walk
  /// that still hits the bound makes the snapshot terminal rather than partial:
  /// [`EvaluationSnapshot::spawn`] answers nothing, and the names it had left
  /// to follow are reported through [`EvaluationSnapshot::omitted`].
  ///
  /// What is deliberately *not* carried: the compiled style map, the injected
  /// rules, the pending module items, the metadata, the evaluator memo, the
  /// kept function maps, and the module's record of every call it holds. A
  /// snapshot that carried any of them would both cost what this exists to
  /// avoid and make [`EvaluationEffects::harvest`] unable to tell what the
  /// callback wrote from what was already there. The call record is replaced by
  /// [`EvaluationSnapshot::observe_calls_in`], which seeds the calls one body
  /// writes.
  pub fn evaluation_snapshot_for(&self, ids: impl IntoIterator<Item = Id>) -> EvaluationSnapshot {
    let mut small = StateManager::new(StyleXOptions::default());

    // Options first, because the ceilings the walk below reads come from them.
    small.options = self.options.clone();

    // The parsed source and the bytes it was parsed from. The module byte bound
    // every evaluation sizes itself against is `input_source_file`'s own
    // length, so carrying the file carries the bound unchanged -- and the walk
    // below scales its own bounds from the same number.
    small.input_source_file = self.input_source_file.clone();
    small.module_source = self.module_source.clone();

    // The four safety records, shared rather than copied -- each is already
    // behind an `Rc` for exactly this reason.
    small.binding_reassignments = Rc::clone(&self.binding_reassignments);
    small.binding_mutations = Rc::clone(&self.binding_mutations);
    small.binding_deep_mutations = Rc::clone(&self.binding_deep_mutations);
    small.declared_bindings = Rc::clone(&self.declared_bindings);

    // Where the module's calls are written, shared rather than copied.
    //
    // The module's record of every *call expression* is not carried at all --
    // it is a map the size of the file, and the only question asked of it is
    // about the body being folded. See
    // [`EvaluationSnapshot::observe_calls_in`].
    small.call_positions = Rc::clone(&self.call_positions);

    // The StyleX API surface, so `stylex.*` inside the body resolves to the API
    // it resolved to when the body was written.
    //
    // Carried whole, and bounded by the API rather than by the module: the
    // namespace bindings, the role each API name was imported under, and the
    // import sources StyleX itself was imported from. A module with a thousand
    // unrelated imports adds nothing to any of them. What *is* module-sized --
    // the set of every import path the file names, and the authored order of
    // every import in it -- is narrowed below to the imports this snapshot
    // actually carries.
    small.imports = self.imports.clone();
    small.export_id = self.export_id.clone();
    small.cycle = self.cycle;

    // Shared, so a promotion made before the snapshot is still one after it.
    small.evaluation_heap = Rc::clone(&self.evaluation_heap);

    let bytes = self
      .input_source_file
      .as_ref()
      .map_or(0, |source| source.src.len());
    let bounds = snapshot_bounds(bytes);

    let mut reached = FxHashSet::default();
    let mut carried_import_spans: FxHashSet<Span> = FxHashSet::default();
    let mut root_bindings = FxHashSet::default();
    let mut queue: Vec<Id> = ids.into_iter().collect();
    let mut omitted = Vec::new();
    let ceiling = self.evaluation_ceiling();

    while let Some(id) = queue.pop() {
      if !reached.insert(id.clone()) {
        continue;
      }

      if reached.len() > bounds.bindings {
        // Terminal, and said so with the names that were left: the snapshot
        // carries no partial answer, so nothing it holds can be folded against.
        reached.remove(&id);
        omitted.push(id);
        omitted.append(&mut queue);
        break;
      }

      // Built with a dummy span on purpose: both resolvers confirm with
      // `eq_ignore_span`, and the `Id` is what decides. A span taken from the
      // reference site would make the snapshot depend on *where* the name was
      // read rather than on which binding it names.
      let ident = Ident::new(id.0.clone(), DUMMY_SP, id.1);

      // Only the reached names' own root records, rather than the module's set
      // of every root binding in the file.
      if self.is_module_root_binding(&id) {
        root_bindings.insert(id.clone());
      }

      if let Some(declarator) = self.declaration_of(&ident) {
        let declarator = declarator.clone();
        let mut complete = true;

        if let Some(init) = declarator.init.as_deref() {
          complete = collect_referenced_ids(init, ceiling, bounds.references, &mut queue);
        }

        small.push_declaration(declarator);

        // An initializer whose references outran the budget leaves names
        // nobody followed, which is the same terminal answer as outrunning the
        // binding bound: the snapshot may not be folded against.
        if !complete {
          omitted.push(id);
          omitted.append(&mut queue);
          break;
        }
      }

      if let Some((import, _specifier)) = self.import_binding(&ident) {
        // Keyed by the declaration's span rather than by its position, because
        // the position is this module's index and the snapshot's list is its
        // own. One import binding several reached names is carried once.
        if carried_import_spans.insert(import.span) {
          small.push_top_import(import.clone());
        }
      }
    }

    small.adopt_module_root_bindings(root_bindings);

    // The hoisted-declaration records, narrowed to the names the walk reached.
    //
    // `class_names` and `function_names` are keyed by `Id` and hold one entry
    // per hoisted declaration in the file, so a module with a thousand helper
    // functions carried a thousand entries for a callback that reads one of
    // them. Only a reached name can be asked about -- `declared_as` is reached
    // through resolving a reference, and a reference the snapshot cannot
    // resolve never gets that far -- so the entries for every other name are
    // weight with no reader.
    //
    // Narrowed rather than dropped: the record is what tells a reference bound
    // by a `class` or a `function` apart from one bound by a declarator, and
    // losing it for a reached name would make that reference resolve to the
    // wrong kind. Both maps are filtered by the same set, in one pass each.
    small.declarations_state.class_names = self
      .declarations_state
      .class_names
      .iter()
      .filter(|(id, _)| reached.contains(*id))
      .map(|(id, span)| (id.clone(), *span))
      .collect();
    small.declarations_state.function_names = self
      .declarations_state
      .function_names
      .iter()
      .filter(|(id, _)| reached.contains(*id))
      .map(|(id, span)| (id.clone(), *span))
      .collect();
    // Unsupported patterns/parameters still need their original fault nodes
    // when a stored callback reaches them. Keep only the actual reached Ids.
    small.declarations_state.opaque_faults = self
      .declarations_state
      .opaque_faults
      .iter()
      .filter(|(id, _)| reached.contains(*id))
      .map(|(id, fault)| (id.clone(), *fault))
      .collect();

    // The import paths, narrowed to the imports the snapshot carries plus the
    // ones StyleX itself was imported from.
    //
    // The StyleX sources stay whichever they are: `is_regular_stylex_import`
    // and the namespace reads are what make `stylex.create` resolve inside the
    // body, and they are bounded by the API rather than by the module. Every
    // other path in the set belongs to an import this snapshot did not carry,
    // so nothing left here can ask about it.
    let carried_sources: FxHashSet<String> = small
      .top_imports
      .iter()
      .filter_map(|import| import.src.value.as_str().map(str::to_owned))
      .collect();
    let stylex_sources: FxHashSet<String> = self
      .stylex_imports()
      .iter()
      .map(|source| source.get_import_str().to_owned())
      .collect();

    small.imports.import_paths = self
      .imports
      .import_paths
      .iter()
      .filter(|path| carried_sources.contains(*path) || stylex_sources.contains(*path))
      .cloned()
      .collect();

    small.imports.authored_import_order = self
      .imports
      .authored_import_order
      .iter()
      .filter(|span| carried_import_spans.contains(span))
      .copied()
      .collect();

    EvaluationSnapshot {
      state: small,
      reached,
      omitted,
      bounds,
    }
  }
}

/// Every identifier `init` reads, appended to `queue`.
///
/// An over-approximation on purpose: a name that turns out to bind nothing
/// costs one failed probe, where a name the walk missed costs a reference that
/// stops resolving inside the callback. What it does *not* over-approximate is
/// the two shapes where an identifier is a spelling rather than a reference --
/// a dotted property and a plain object key -- because those name no binding in
/// any module and following them would carry declarations at random.
///
/// Bounded in both directions: at `references`, which [`snapshot_bounds`]
/// scaled from the module's own size, and no deeper than `ceiling` levels of
/// expression.
fn collect_referenced_ids(
  init: &Expr,
  ceiling: usize,
  references: usize,
  queue: &mut Vec<Id>,
) -> bool {
  let mut walk = Referenced {
    queue,
    budget: references,
    depth: 0,
    ceiling,
    complete: true,
  };

  init.visit_with(&mut walk);

  walk.complete
}

struct Referenced<'a> {
  queue: &'a mut Vec<Id>,
  budget: usize,
  depth: usize,
  ceiling: usize,
  /// Whether every reference and every level was read. A walk that stopped at
  /// either bound left names nobody followed, which is what makes the snapshot
  /// around it terminal.
  complete: bool,
}

impl Referenced<'_> {
  /// One level of the descent, refused at the ceiling.
  ///
  /// The walk recurses on the bare thread stack, so the ceiling is what keeps a
  /// deeply nested initializer from overflowing it here -- the same bound the
  /// evaluator's own descent runs under, read off the same options.
  fn one_level_of<N: VisitWith<Self> + ?Sized>(&mut self, node: &N) {
    if self.budget == 0 {
      self.complete = false;
      return;
    }

    if self.depth >= self.ceiling {
      self.complete = false;
      return;
    }

    self.depth += 1;
    node.visit_children_with(self);
    self.depth -= 1;
  }
}

impl Visit for Referenced<'_> {
  fn visit_expr(&mut self, expr: &Expr) {
    self.one_level_of(expr);
  }

  fn visit_ident(&mut self, ident: &Ident) {
    if self.budget == 0 {
      self.complete = false;
      return;
    }

    self.budget -= 1;
    self.queue.push(ident.to_id());
  }

  /// A dotted property is a spelling, not a reference: `a.b` reads `a`. A
  /// computed one is an expression and is walked.
  fn visit_member_expr(&mut self, member: &MemberExpr) {
    member.obj.visit_with(self);

    if let MemberProp::Computed(key) = &member.prop {
      key.expr.visit_with(self);
    }
  }

  /// The same for an object key, and for the same reason. A shorthand property
  /// is both a key and a reference, and SWC spells it as `Prop::Shorthand`
  /// holding an identifier, so it is read here rather than skipped.
  fn visit_prop(&mut self, prop: &Prop) {
    match prop {
      Prop::Shorthand(ident) => self.visit_ident(ident),
      Prop::KeyValue(key_value) => {
        if let PropName::Computed(key) = &key_value.key {
          key.expr.visit_with(self);
        }

        key_value.value.visit_with(self);
      },
      other => other.visit_children_with(self),
    }
  }
}

/// The calls one body writes, recorded on the state it will fold against.
///
/// Only the calls: what the state is asked afterwards is whether a member read
/// is the callee of one, and that question is answered by
/// `CallExpressionState`'s own index -- which this fills through
/// `StateManager::add_call_expression`, the writer that maintains it. The
/// module's own record of every call in the file is not carried, so what a
/// snapshot knows about calls is exactly the body it was shown.
///
/// Bounded by the same ceiling the reference walk runs under, since it descends
/// on the bare thread stack too.
struct SeenCalls<'a> {
  state: &'a mut StateManager,
  depth: usize,
  ceiling: usize,
}

impl SeenCalls<'_> {
  fn one_level_of<N: VisitWith<Self> + ?Sized>(&mut self, node: &N) {
    if self.depth >= self.ceiling {
      return;
    }

    self.depth += 1;
    node.visit_children_with(self);
    self.depth -= 1;
  }
}

impl Visit for SeenCalls<'_> {
  fn visit_expr(&mut self, expr: &Expr) {
    self.one_level_of(expr);
  }

  fn visit_call_expr(&mut self, call: &CallExpr) {
    self.state.add_call_expression(call);
    self.one_level_of(call);
  }
}

// ==================== what an invocation did, and when it counts ====================

/// One thing an invocation did to the module around it.
///
/// Only the effects a *fold* can have. A producer registration is not among
/// them: `register_styles` writes the metadata, the placement and the call-site
/// rewrite together, and replaying those out of order is how a `@keyframes`
/// block ends up behind the rule that names it. A callback that reaches one is
/// reported rather than journalled -- see the module's own note on gaps.
#[derive(Clone, Debug)]
pub enum EvaluationEffect {
  /// A rule a nested producer filed on the state rather than registering on its
  /// own, which the producer holding it carries.
  Rule {
    key: RuleKey,
    style: Rc<InjectableStyleKind>,
  },
  /// The filing itself, which the evaluator memo reads to decline to remember a
  /// fold that produced a rule. Counted per filing, like the counter it
  /// replays into.
  NestedRuleFiled,
  /// A theme side-effect import. `at` is the span of the import declaration a
  /// confident read kept, where there was one, and `None` is the plain queue.
  ///
  /// The span rather than the slot, because replay goes back through the two
  /// public writers -- `queue_theme_import_at` and `queue_theme_import_if_absent`
  /// -- and those are what the slot is derived from. Carrying the slot would
  /// mean reaching the private writer underneath them and keeping a second copy
  /// of the derivation here.
  ThemeImport { at: Option<Span>, item: ModuleItem },
}

/// An ordered, owned record of what one or more invocations did.
///
/// Owned rather than borrowed, because the whole point is that it outlives the
/// state it was produced against: a callback an owner retained runs against a
/// snapshot, and what it did has to reach whichever transform is active when it
/// runs, not the one that built it.
///
/// Replay is **all or nothing** and **once**. [`Self::replay_into`] consumes
/// the journal, so there is no second replay to guard against, and a journal
/// dropped without being replayed -- which is what a terminal callback error
/// leaves behind -- writes nothing at all.
#[derive(Debug, Default)]
pub struct EvaluationEffects {
  entries: Vec<EvaluationEffect>,
}

impl EvaluationEffects {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn is_empty(&self) -> bool {
    self.entries.is_empty()
  }

  pub fn len(&self) -> usize {
    self.entries.len()
  }

  /// What was recorded, in the order it was recorded.
  pub fn effects(&self) -> &[EvaluationEffect] {
    &self.entries
  }

  pub fn record(&mut self, effect: EvaluationEffect) {
    self.entries.push(effect);
  }

  pub fn record_rule(&mut self, key: RuleKey, style: Rc<InjectableStyleKind>) {
    self.record(EvaluationEffect::Rule { key, style });
  }

  pub fn record_nested_rule_filed(&mut self) {
    self.record(EvaluationEffect::NestedRuleFiled);
  }

  pub fn record_theme_import(&mut self, at: Option<Span>, item: ModuleItem) {
    self.record(EvaluationEffect::ThemeImport { at, item });
  }

  /// Appends `other`'s entries after this journal's own, keeping both orders.
  pub fn absorb(&mut self, other: Self) {
    self.entries.extend(other.entries);
  }

  /// Everything `spawned` recorded, taken off it.
  ///
  /// A snapshot's state starts with no rules, no filings and no pending items,
  /// so whatever is on it afterwards is this invocation's own -- which is why
  /// the snapshot carries none of those three. The state is drained, so a
  /// second harvest of the same invocation answers nothing: there is no reading
  /// of this that records an effect twice.
  ///
  /// Order is preserved *per destination*, which is the order that is
  /// observable: the rules keep their filing order, the filings keep their
  /// count, and the theme imports keep their queue order. The three destinations
  /// are independent ordered buffers, so how they interleaved with each other
  /// during the invocation is not something the module body can see. A caller
  /// that needs the exact global order records through [`publish_effect`]
  /// instead, which keeps one sequence.
  pub fn harvest(spawned: &mut StateManager) -> Self {
    Self::harvest_past(spawned, None)
  }

  /// The same, minus whatever `published` already carried away.
  ///
  /// A rule or a filing that went through [`StateManager::file_evaluation_rule`]
  /// or [`StateManager::note_evaluation_rule_filed`] while a callback's
  /// publication ledger was open reached the collector *at the moment it
  /// happened*, which is the whole point of those writers: the order the
  /// collector holds is the order the effects occurred in, including a nested
  /// call's effects landing between two of this body's own.
  ///
  /// The local map and counter still hold them, because they are the cache
  /// this state answers its own reads from. So the end-of-invocation harvest
  /// has to take the difference rather than everything, or every published
  /// effect would be journalled twice and replayed twice.
  ///
  /// What the ledger does *not* track is journalled here as usual: a theme
  /// import is queued rather than published, so it is carried exactly once, by
  /// this harvest.
  fn harvest_past(spawned: &mut StateManager, published: Option<&PublicationLedger>) -> Self {
    let mut journal = Self::new();

    for (key, style) in std::mem::take(&mut spawned.other_injected_css_rules) {
      let already = published.is_some_and(|ledger| ledger.carried_rule(&key, &style));

      if !already {
        journal.record_rule(key, style);
      }
    }

    let filed = std::mem::take(&mut spawned.nested_rules_filed);
    let unpublished = filed.saturating_sub(published.map_or(0, PublicationLedger::carried_filings));

    for _ in 0..unpublished {
      journal.record_nested_rule_filed();
    }

    for pending in std::mem::take(&mut spawned.pending_module_items) {
      match pending.slot {
        InsertionSlot::ThemeImports => journal.record_theme_import(None, pending.item),
        InsertionSlot::ThemeImportAt(span) => journal.record_theme_import(Some(span), pending.item),
        // Every other slot belongs to a producer registration, which this
        // journal does not carry. Put back rather than dropped, so a caller
        // that does know what to do with it still can.
        slot => spawned.pending_module_items.push(PendingInsertion {
          slot,
          item: pending.item,
        }),
      }
    }

    journal
  }

  /// Replays everything, in order, through the registration paths the module
  /// already has.
  ///
  /// Consuming, which is what makes a double replay unwritable rather than
  /// merely unwise. Each arm goes through the same **public** entry point the
  /// original producer used, so the dedup, the ordering and the slot
  /// resolution are the module's own and not a second copy of them: the rule
  /// map is keyed as it is keyed everywhere, the filing counter is bumped
  /// through its own writer, and a theme import goes back through the two
  /// writers that hold the queue's dedup.
  pub fn replay_into(self, state: &mut StateManager) {
    for effect in self.entries {
      match effect {
        // Through the publishing writers, not past them. This is what puts a
        // nested call's effects where they belong in the outer callback's
        // sequence: the replay happens when the nested call returns, so the
        // outer collector receives them then -- before the outer body files
        // whatever it files next, rather than all together at the end.
        EvaluationEffect::Rule { key, style } => state.file_evaluation_rule(key, style),
        EvaluationEffect::NestedRuleFiled => state.note_evaluation_rule_filed(),
        // The plain queue, which derives its own slot.
        EvaluationEffect::ThemeImport { at: None, item } => {
          state.queue_theme_import_if_absent(item);
        },
        // The pinned one. `queue_theme_import_at` builds the queued item from
        // the declaration's own source and takes the slot from its span, so
        // handing it a declaration carrying both reproduces the original
        // queue exactly -- and the specifiers it does not read are left empty
        // rather than invented.
        EvaluationEffect::ThemeImport {
          at: Some(span),
          item,
        } => {
          match theme_import_source(&item) {
            Some(src) => state.queue_theme_import_at(&ImportDecl {
              span,
              specifiers: vec![],
              src: Box::new(src),
              type_only: false,
              with: None,
              phase: ImportPhase::Evaluation,
            }),
            // An item with no import source is not a theme import, and there
            // is no public writer for one. Queued as the plain kind rather
            // than dropped: a side-effect import in the wrong slot still
            // defines the names it defines, where a missing one does not.
            None => state.queue_theme_import_if_absent(item),
          }
        },
      }
    }
  }
}

/// The source a queued theme import names.
fn theme_import_source(item: &ModuleItem) -> Option<Str> {
  item
    .as_module_decl()
    .and_then(|decl| decl.as_import())
    .map(|import| (*import.src).clone())
}

thread_local! {
  /// The journal an invocation on this thread is publishing into, if one is
  /// installed.
  ///
  /// A thread-local rather than something threaded through the evaluator,
  /// because the producer that files a rule is several frames below the call
  /// that installed the sink and none of the frames between it know a sink
  /// exists.
  ///
  /// Deliberately **not** the native evaluation budget's scope. A retained
  /// callback invoked during a later transform runs under whatever budget that
  /// transform installed, or under an isolated one, and the effects it produces
  /// still belong to that transform. Tying the sink to the budget would send
  /// them to whichever evaluation happened to be open instead.
  static COLLECTING: RefCell<Option<Rc<RefCell<EvaluationEffects>>>> =
    const { RefCell::new(None) };
}

/// The journal installed for the current scope, for as long as it is held.
///
/// RAII, and the only way to install one: the thread-local is private, so a
/// sink cannot be left behind by a path that forgot to clear it. A panic
/// unwinding out of the scope restores the previous sink on the way past.
///
/// Nested scopes are a stack: an inner one replaces the outer for its lifetime
/// and puts it back, so a callback that invokes another callback journals into
/// its own scope and the outer journal is unaffected.
pub struct ScopedEffects {
  journal: Rc<RefCell<EvaluationEffects>>,
  previous: Option<Rc<RefCell<EvaluationEffects>>>,
}

impl ScopedEffects {
  /// Installs a fresh journal for this thread and answers the handle that owns
  /// it.
  pub fn install() -> Self {
    let journal = Rc::new(RefCell::new(EvaluationEffects::new()));
    let previous = COLLECTING.with(|slot| slot.borrow_mut().replace(Rc::clone(&journal)));

    Self { journal, previous }
  }

  /// How many effects have been published so far, without taking them.
  pub fn len(&self) -> usize {
    self.journal.borrow().len()
  }

  pub fn is_empty(&self) -> bool {
    self.journal.borrow().is_empty()
  }

  /// Adds what an invocation left on its own state to this scope's journal.
  ///
  /// For the harvesting half of the collection: a producer that writes to the
  /// state it was handed is journalled here, and one that publishes directly is
  /// journalled by [`publish_effect`]. Both land in one sequence.
  pub fn absorb(&self, effects: EvaluationEffects) {
    self.journal.borrow_mut().absorb(effects);
  }

  /// Uninstalls the sink and answers what was collected.
  ///
  /// The success path. A caller that reaches here has a value to return, so the
  /// effects behind it are real and are replayed by whoever owns the state.
  pub fn finish(self) -> EvaluationEffects {
    // `self` is consumed, so `Drop` runs at the end of this function and puts
    // the previous sink back. What is taken here is the journal's contents; the
    // `Rc` the scope held is dropped with it.
    std::mem::take(&mut *self.journal.borrow_mut())
  }

  /// Uninstalls the sink and throws the journal away.
  ///
  /// The failure path, spelled out rather than left to a bare `drop`: a
  /// terminal callback error must not write a partial module, and a reader of
  /// the call site should see which of the two outcomes is being taken.
  pub fn discard(self) {}
}

impl Drop for ScopedEffects {
  fn drop(&mut self) {
    COLLECTING.with(|slot| *slot.borrow_mut() = self.previous.take());
  }
}

impl std::fmt::Debug for ScopedEffects {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("ScopedEffects")
      .field("collected", &self.journal.borrow().len())
      .field("nested", &self.previous.is_some())
      .finish()
  }
}

/// Records `effect` against the installed sink, answering whether there was
/// one.
///
/// `false` is a caller outside any collecting scope -- an ordinary callback
/// folding against the caller's own state, which needs no journal because it
/// already wrote where the effect belongs. So a producer may call this
/// unconditionally and let the scope decide.
///
/// The borrow is taken and released inside this call. Nothing user-written runs
/// while it is held, which is what keeps a producer that publishes from inside
/// another publish from panicking on a re-entrant borrow.
pub fn publish_effect(effect: EvaluationEffect) -> bool {
  let journal = COLLECTING.with(|slot| slot.borrow().as_ref().map(Rc::clone));

  match journal {
    Some(journal) => {
      journal.borrow_mut().record(effect);
      true
    },
    None => false,
  }
}

/// Whether an invocation on this thread is collecting effects rather than
/// writing them where it stands.
pub fn effects_are_collected() -> bool {
  COLLECTING.with(|slot| slot.borrow().is_some())
}

#[cfg(test)]
#[path = "tests/evaluation_scope_test.rs"]
mod evaluation_scope_test;

// ============ the ledger one compiler callback publishes against ============

/// What one compiler callback has already handed to the collector.
///
/// A rule is marked by **both** its key and the identity of the style behind
/// it. The key alone would be wrong twice over: two producers can file the same
/// key with different styles, and the same style can be re-filed under a key
/// the map already holds. Holding the pair means a re-file of a genuinely
/// different style is published again, as it must be, and a re-file of the same
/// one is not.
///
/// A filing is a count, because that is all a filing is -- the memo reads how
/// many happened, never which.
///
/// Bounded by the callback it belongs to: it is created when the callback's
/// publication scope opens and dropped when it closes, so what it holds is one
/// invocation's effects and never a module's.
#[derive(Default)]
pub struct PublicationLedger {
  rules: FxHashSet<(RuleKey, usize)>,
  filings: u64,
}

impl std::fmt::Debug for PublicationLedger {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("PublicationLedger")
      .field("rules", &self.rules.len())
      .field("filings", &self.filings)
      .finish()
  }
}

/// The identity of one filed style, as the ledger keys it.
///
/// The allocation's address, which is what tells two styles apart when their
/// keys agree. Read through `Rc::as_ptr` rather than by comparing the values,
/// because a style is compared for *sameness* here and not for equality: two
/// equal styles filed separately are two effects.
fn style_identity(style: &Rc<InjectableStyleKind>) -> usize {
  Rc::as_ptr(style) as *const () as usize
}

impl PublicationLedger {
  /// Whether this rule already reached the collector.
  pub fn carried_rule(&self, key: &RuleKey, style: &Rc<InjectableStyleKind>) -> bool {
    self.rules.contains(&(key.clone(), style_identity(style)))
  }

  /// How many filings already reached it.
  pub fn carried_filings(&self) -> u64 {
    self.filings
  }

  pub fn rules_carried(&self) -> usize {
    self.rules.len()
  }

  fn mark_rule(&mut self, key: RuleKey, identity: usize) {
    self.rules.insert((key, identity));
  }

  fn mark_filing(&mut self) {
    self.filings = self.filings.saturating_add(1);
  }
}

thread_local! {
  /// The publication ledger the compiler callback running on this thread is
  /// marking against, if one is installed.
  ///
  /// Beside [`COLLECTING`] rather than inside it, because the two answer
  /// different questions and open at different times: the collector says
  /// *where* an effect goes, the ledger says *what this callback has already
  /// sent there*. A collector with no ledger is the ordinary case -- an effect
  /// written straight onto the caller's own state, journalled at the end -- and
  /// must keep behaving exactly as it did.
  ///
  /// `Rc`-owned, like the collector, and for the same reason: nothing here may
  /// hold a pointer into a `StateManager`. A callback's state is spawned per
  /// invocation and dropped with it, where the ledger has to outlive each
  /// write it records.
  static PUBLISHING: RefCell<Option<Rc<RefCell<PublicationLedger>>>> =
    const { RefCell::new(None) };
}

/// The publication scope one compiler callback runs inside.
///
/// RAII, and the only way to install a ledger. Nested scopes are a stack, as
/// the collector's are: a callback invoked from inside another callback marks
/// against its own ledger, and the outer one is untouched — which is what keeps
/// the inner call's harvest from filtering the outer call's effects.
pub struct CallbackPublication {
  ledger: Rc<RefCell<PublicationLedger>>,
  previous: Option<Rc<RefCell<PublicationLedger>>>,
}

impl CallbackPublication {
  /// Opens a ledger for this thread and answers the handle that owns it.
  pub fn install() -> Self {
    let ledger = Rc::new(RefCell::new(PublicationLedger::default()));
    let previous = PUBLISHING.with(|slot| slot.borrow_mut().replace(Rc::clone(&ledger)));

    Self { ledger, previous }
  }

  /// How many distinct rules this callback has published so far.
  pub fn rules_published(&self) -> usize {
    self.ledger.borrow().rules_carried()
  }

  /// How many filings it has published so far.
  pub fn filings_published(&self) -> u64 {
    self.ledger.borrow().carried_filings()
  }

  /// What `spawned` is still holding that the collector has not already been
  /// given.
  ///
  /// The end-of-invocation harvest. Everything this callback published as it
  /// went is filtered out, because it is already in the collector and in the
  /// right place in its sequence; everything the ledger does not track -- a
  /// theme import above all -- is carried here, exactly once.
  ///
  /// The ledger's borrow is released before the state is touched, so a harvest
  /// cannot deadlock against a writer that publishes while it runs.
  pub fn harvest(&self, spawned: &mut StateManager) -> EvaluationEffects {
    // Whatever an immutable host operation queued and no writer has let out
    // yet. Flushed before the ledger is read, so it is published in its own
    // place and then filtered here like everything else -- rather than being
    // journalled now, which would put it after effects it preceded.
    spawned.replay_queued_evaluation_effects();

    let ledger = self.ledger.borrow();

    EvaluationEffects::harvest_past(spawned, Some(&ledger))
  }
}

impl Drop for CallbackPublication {
  fn drop(&mut self) {
    PUBLISHING.with(|slot| *slot.borrow_mut() = self.previous.take());
  }
}

impl std::fmt::Debug for CallbackPublication {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("CallbackPublication")
      .field("ledger", &*self.ledger.borrow())
      .field("nested", &self.previous.is_some())
      .finish()
  }
}

/// Whether a compiler callback on this thread is publishing its effects as it
/// produces them.
pub fn effects_are_published() -> bool {
  PUBLISHING.with(|slot| slot.borrow().is_some())
}

/// Hands one effect to the collector and marks it on the ledger, where both
/// are open.
///
/// `false` where either is closed, which is the ordinary path: an effect
/// written onto the caller's own state, carried by the end-of-invocation
/// harvest rather than published as it happens.
///
/// Both borrows are taken and released inside this call, and nothing
/// user-written runs while either is held.
fn publish_against_the_ledger(
  effect: EvaluationEffect,
  mark: impl FnOnce(&mut PublicationLedger),
) -> bool {
  let ledger = PUBLISHING.with(|slot| slot.borrow().as_ref().map(Rc::clone));
  let Some(ledger) = ledger else {
    return false;
  };

  let journal = COLLECTING.with(|slot| slot.borrow().as_ref().map(Rc::clone));
  let Some(journal) = journal else {
    return false;
  };

  journal.borrow_mut().record(effect);
  mark(&mut ledger.borrow_mut());

  true
}

impl StateManager {
  /// Files one rule the evaluation produced, and publishes it where a callback
  /// is collecting.
  ///
  /// Two destinations, deliberately. The local map is the cache this state
  /// answers its own reads from and is written whether or not anybody is
  /// collecting -- a producer that reads back what it filed must find it. The
  /// collector is where the effect goes to reach the transform that will
  /// outlive this invocation, and it is written *now* rather than at the end,
  /// so the sequence it holds is the sequence things happened in.
  ///
  /// Every writer of a rule inside an evaluation should come through here. One
  /// that writes the map directly still works -- the harvest carries what the
  /// ledger did not -- but its effect lands at the end of the sequence rather
  /// than at its own place in it.
  pub fn file_evaluation_rule(&mut self, key: RuleKey, style: Rc<InjectableStyleKind>) {
    // Anything an immutable host operation queued happened *before* this
    // write, so it is published before it. This is what keeps a nested host
    // call's effects in their place: the nested op could only queue, and the
    // next write by the outer body is what lets them out -- ahead of itself.
    self.replay_queued_evaluation_effects();

    self
      .other_injected_css_rules
      .insert(key.clone(), Rc::clone(&style));

    // Read before the move, so the mark costs the key's own clone and no
    // second handle on the style.
    let marked = key.clone();
    let identity = style_identity(&style);

    // The answer is the scope's business rather than the writer's: an effect
    // nobody is collecting is already where it belongs, on the state.
    let _published =
      publish_against_the_ledger(EvaluationEffect::Rule { key, style }, move |ledger| {
        ledger.mark_rule(marked, identity)
      });
  }

  /// Records that one nested rule was filed, and publishes the filing where a
  /// callback is collecting.
  ///
  /// The counter half of [`Self::file_evaluation_rule`], and the same two
  /// destinations: the local counter the memo compares across a fold, and the
  /// collector's own sequence.
  pub fn note_evaluation_rule_filed(&mut self) {
    self.replay_queued_evaluation_effects();
    self.note_nested_rule_filed();

    let _published = publish_against_the_ledger(
      EvaluationEffect::NestedRuleFiled,
      PublicationLedger::mark_filing,
    );
  }
}

// ==================== a root job of its own ====================

/// Runs `job` with neither a collector nor a ledger installed, restoring both
/// afterwards.
///
/// A root isolation, for a transform that begins inside another one's scope: a
/// retained callback invoked during a later transform runs that transform's
/// work, and the effects it produces belong there rather than in whichever
/// collector happened to be open on this thread. Without this they would leak
/// outward into a sink that will be replayed into an unrelated module.
///
/// **Separate from the native evaluation budget**, like the collector itself.
/// A budget bounds how much work an evaluation may do; this bounds where its
/// effects may go, and the two are opened by different callers at different
/// times. Tying them together would make an isolated budget silently discard
/// effects, or an isolated effect scope silently reset a budget.
///
/// Restored on the way out of a panic as well as a return, because the guard
/// owns the restoration rather than the closure.
pub fn with_isolated_evaluation_effects<T>(job: impl FnOnce() -> T) -> T {
  let _root = IsolatedEffects {
    collector: COLLECTING.with(|slot| slot.borrow_mut().take()),
    ledger: PUBLISHING.with(|slot| slot.borrow_mut().take()),
  };

  job()
}

/// What [`with_isolated_evaluation_effects`] took, and puts back.
struct IsolatedEffects {
  collector: Option<Rc<RefCell<EvaluationEffects>>>,
  ledger: Option<Rc<RefCell<PublicationLedger>>>,
}

impl Drop for IsolatedEffects {
  fn drop(&mut self) {
    COLLECTING.with(|slot| *slot.borrow_mut() = self.collector.take());
    PUBLISHING.with(|slot| *slot.borrow_mut() = self.ledger.take());
  }
}

// ==================== what no journal can carry ====================

/// Output an invocation produced that no [`EvaluationEffects`] carries.
///
/// A producer registration is the whole of it: `register_styles` writes the
/// metadata, the placement and the call-site rewrite together, and replaying
/// those out of order is how a `@keyframes` block lands behind the rule that
/// names it. This journal does not reimplement that ordering, and inventing one
/// here would be worse than refusing.
///
/// So a callback that produced any of this **cannot be replayed**, and the
/// caller has to fail closed rather than carry on. The failure that matters is
/// silent: a class name reaches the stylesheet and the CSS defining it does
/// not, so the page renders unstyled with nothing reported anywhere.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UnreplayableEffects {
  /// Items queued for a slot that is not a theme import -- a producer's own
  /// placement.
  pub placements: usize,
  /// Namespaces a `create` call registered.
  pub registered_styles: usize,
  /// Rules the host collects as metadata.
  pub metadata_rules: usize,
}

impl UnreplayableEffects {
  pub fn total(&self) -> usize {
    self.placements + self.registered_styles + self.metadata_rules
  }
}

/// What `spawned` is holding that no journal can carry, or `None` where it is
/// holding none of it.
///
/// Asked **after** the harvest, which is when the theme imports have already
/// been taken and anything left in the pending items is a producer's own
/// placement. A caller that gets `Some` refuses the fold; a caller that gets
/// `None` has a journal that carries everything the invocation produced.
///
/// Read-only, and it invents nothing: it reports the three shapes a producer
/// registration leaves behind and names no ordering for them, because the
/// ordering is what this journal deliberately does not own.
pub fn unreplayable_effects(spawned: &StateManager) -> Option<UnreplayableEffects> {
  let left = UnreplayableEffects {
    placements: spawned.pending_module_items.len(),
    registered_styles: spawned.style_map.len(),
    metadata_rules: spawned
      .metadata()
      .values()
      .map(IndexSet::len)
      .sum::<usize>(),
  };

  (left.total() > 0).then_some(left)
}

// ============ effects an immutable host operation could not write ============

/// The smallest queue any heap offers.
///
/// A host operation reading through `&StateManager` cannot write its effects,
/// so they wait for a writer. The wait is bounded because the queue is the one
/// place in this module where a producer can accumulate without a writer ever
/// arriving: a read-only fold that files a thousand rules and is then
/// discarded would otherwise grow it for the lifetime of the heap.
///
/// A **floor**, not a cap. A heap reached with no state behind it -- a direct
/// queue in a test, a promotion-only heap -- gets this and nothing more,
/// because there is nothing to size against. A heap belonging to a live state
/// is sized from that state, by [`StateManager::pending_effects_limit`].
pub const MIN_PENDING_EFFECTS: usize = 4096;

/// The most any queue will hold, however large the module.
///
/// Past this the queue is not a module's output but a loop's, and a loop's
/// answer is terminal. Sixty-five thousand events is a stylesheet no author
/// writes and a bound a runaway reaches in a moment.
pub const MAX_PENDING_EFFECTS_LIMIT: usize = 65_536;

/// Four events per folded entry.
///
/// A folded entry can legitimately become more than one effect -- a keyframes
/// block files its rule and notes its filing, which is two, and a nested
/// condition can double that again. Four is that doubling with room, so a
/// module at its entry ceiling is not refused for being at it.
const EFFECTS_PER_ENTRY: usize = 4;

/// Two events per source byte.
///
/// The same reading the snapshot bounds take: a module's own size is the only
/// O(1) measure of how much it can legitimately produce, and a byte of source
/// cannot become more than a couple of events without a loop in between.
const EFFECTS_PER_SOURCE_BYTE: usize = 2;

impl EvaluationHeap {
  /// Appends `effects` to the queue, or refuses them for want of room.
  ///
  /// Refusal is remembered rather than reported here, because the caller is a
  /// read path with nowhere to put an error. The state's own
  /// [`StateManager::pending_effects_overflowed`] is what a writer asks before
  /// it trusts a replay.
  ///
  /// `limit` comes from the caller, because the heap cannot derive one: it
  /// holds promotions and a queue, not the module they belong to. A heap
  /// reached directly gets [`MIN_PENDING_EFFECTS`] through
  /// [`Self::queue_effects_at_the_floor`].
  fn queue_effects(&mut self, effects: EvaluationEffects, limit: usize) {
    let incoming = effects.len();

    if self.pending.len().saturating_add(incoming) > limit {
      // Nothing partial: half a producer's rules replayed is worse than none,
      // because none is visible and half is a stylesheet missing a line.
      self.pending_overflowed = true;

      return;
    }

    self.pending.extend(effects.into_entries());
  }

  /// Queues against the floor, for a heap with no state to size against.
  pub fn queue_effects_at_the_floor(&mut self, effects: EvaluationEffects) {
    self.queue_effects(effects, MIN_PENDING_EFFECTS);
  }

  /// Takes the queue, leaving it empty.
  ///
  /// Emptying is what makes a flush safe to re-enter: the writers flush before
  /// they write, and a replay goes back through those writers, so the second
  /// flush must find nothing. The overflow mark is **not** cleared -- it is a
  /// property of what this heap has already lost, not of what it is holding.
  fn take_pending(&mut self) -> EvaluationEffects {
    EvaluationEffects::from_entries(std::mem::take(&mut self.pending))
  }

  /// How many effects are waiting.
  pub fn pending_effects(&self) -> usize {
    self.pending.len()
  }

  /// Whether the queue ever refused an effect.
  pub fn pending_overflowed(&self) -> bool {
    self.pending_overflowed
  }
}

impl EvaluationEffects {
  /// The journal's entries, consumed.
  fn into_entries(self) -> Vec<EvaluationEffect> {
    self.entries
  }

  /// A journal over `entries`, in their order.
  fn from_entries(entries: Vec<EvaluationEffect>) -> Self {
    Self { entries }
  }
}

impl StateManager {
  /// Hands `effects` to the heap, to be replayed by the next writer.
  ///
  /// Takes `&self`, which is the whole point: a host read, a coercion or a
  /// materialisation holds the state immutably and cannot register anything,
  /// and threading mutability through those paths would change signatures
  /// across the evaluator for a case that happens at one boundary. The heap is
  /// already shared through an `Rc<RefCell<_>>`, so the queue rides it.
  ///
  /// The borrow is taken and released inside this call. Nothing user-written
  /// runs while it is held.
  pub fn queue_evaluation_effects(&self, effects: EvaluationEffects) {
    if effects.is_empty() {
      return;
    }

    let limit = self.pending_effects_limit();

    self
      .evaluation_heap
      .borrow_mut()
      .queue_effects(effects, limit);
  }

  /// How many effects this state's queue will hold.
  ///
  /// Scaled from the module, because a blanket number is wrong in both
  /// directions: a stylesheet of two thousand keyframes is two events each
  /// before anything nests, which a four-thousand cap refuses for being large
  /// rather than for looping, while the same cap is far too generous for a
  /// twenty-line file. So the limit is the largest of three readings and then
  /// clamped:
  ///
  /// - [`MIN_PENDING_EFFECTS`], the floor, so a tiny module still has room to
  ///   work in;
  /// - the entry ceiling times [`EFFECTS_PER_ENTRY`], so a fold that is
  ///   allowed to produce that many entries is allowed to publish what they
  ///   become;
  /// - the source's own byte length times [`EFFECTS_PER_SOURCE_BYTE`], which
  ///   is the same measure the snapshot bounds take.
  ///
  /// Then [`MAX_PENDING_EFFECTS_LIMIT`], because past that it is a loop
  /// whatever the module's size says.
  ///
  /// **O(1).** Three reads and some arithmetic: the ceiling is a clamped
  /// option, the byte length is the source's own `len`, and nothing here walks
  /// the module or touches a native counter. Spent fuel is not refunded and
  /// not consulted.
  pub fn pending_effects_limit(&self) -> usize {
    let from_entries = self.entry_ceiling().saturating_mul(EFFECTS_PER_ENTRY);
    let from_source = self
      .input_source_file
      .as_ref()
      .map_or(0, |source| source.src.len())
      .saturating_mul(EFFECTS_PER_SOURCE_BYTE);

    MIN_PENDING_EFFECTS
      .max(from_entries)
      .max(from_source)
      .min(MAX_PENDING_EFFECTS_LIMIT)
  }

  /// Replays whatever was queued, through the ordinary writers.
  ///
  /// The heap borrow is taken, the queue moved out of it, and the borrow
  /// **dropped before the replay begins** -- the replay calls back into the
  /// writers, which read the heap themselves, and holding the borrow across
  /// that would panic the moment a replayed effect reached one.
  ///
  /// Re-entrant by construction rather than by a flag: the queue is emptied
  /// before anything is replayed, so the flush each writer performs finds
  /// nothing and the recursion is one level deep.
  pub fn replay_queued_evaluation_effects(&mut self) {
    let heap = Rc::clone(&self.evaluation_heap);
    // The `RefMut` is a temporary of this statement and dies with it, so the
    // borrow is already released by the time the replay below runs.
    let pending = heap.borrow_mut().take_pending();

    if pending.is_empty() {
      return;
    }

    pending.replay_into(self);
  }

  /// How many effects are waiting on this state's heap.
  pub fn pending_evaluation_effects(&self) -> usize {
    self.evaluation_heap.borrow().pending_effects()
  }

  /// Whether this state's heap ever refused a queued effect.
  ///
  /// Terminal: a caller that sees `true` has lost output it cannot recover,
  /// and must refuse the fold rather than register a class name whose CSS went
  /// missing -- the same fail-closed reading as
  /// [`unreplayable_effects`].
  pub fn pending_effects_overflowed(&self) -> bool {
    self.evaluation_heap.borrow().pending_overflowed()
  }
}

/// Publishes one completed callback batch into the active owner without ever
/// borrowing that owner's StateManager. False means no owner exists.
pub fn publish_effects(effects: EvaluationEffects) -> bool {
  let journal = COLLECTING.with(|slot| slot.borrow().as_ref().cloned());
  match journal {
    Some(journal) => {
      journal.borrow_mut().absorb(effects);
      true
    },
    None => false,
  }
}
