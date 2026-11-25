//! Owner-scoped heap promotion, selective callback snapshots and effect publication. Release ledger borrows before host calls or getters.
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
  /// Effects an immutable host operation produced, waiting for a writer that holds the state mutably.
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

pub const MIN_SNAPSHOT_BINDINGS: usize = 256;

pub const MIN_REFERENCES_PER_INITIALIZER: usize = 4_096;

const FUEL_PER_BINDING: usize = 256;

const FUEL_PER_REFERENCE: usize = 16;

/// What one snapshot of a module this size may walk.
#[derive(Clone, Copy, Debug)]
pub struct SnapshotBounds {
  pub bindings: usize,
  pub references: usize,
}

/// The bounds a module of `bytes` admits.
pub fn snapshot_bounds(bytes: usize) -> SnapshotBounds {
  let fuel = stylex_js::native::MAX_FUEL.max(bytes.saturating_mul(4));

  SnapshotBounds {
    bindings: (fuel / FUEL_PER_BINDING).max(MIN_SNAPSHOT_BINDINGS),
    references: (fuel / FUEL_PER_REFERENCE).max(MIN_REFERENCES_PER_INITIALIZER),
  }
}

/// The part of a module's state a retained callback can still read.
pub struct EvaluationSnapshot {
  state: StateManager,
  reached: FxHashSet<Id>,
  /// The names the walk had left to follow when it hit its bound.
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
  pub fn spawn(&self) -> Option<StateManager> {
    (!self.is_truncated()).then(|| self.state.clone())
  }

  /// Seeds the call shapes a body is about to be folded with.
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

  /// The dependencies the walk did not follow, which is empty unless the bound was hit.
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
  pub fn declaration_records(&self) -> usize {
    self.state.declarations_state.class_names.len()
      + self.state.declarations_state.function_names.len()
      + self.state.declarations_state.opaque_faults.len()
  }

  /// Count retained import paths.
  pub fn import_paths(&self) -> usize {
    self.state.imports.import_paths.len()
  }

  /// Whether the snapshot retained the import path `source`.
  pub fn retains_import_path(&self, source: &str) -> bool {
    self.state.imports.import_paths.contains(source)
  }
}

impl StateManager {
  /// The state a callback that reads `ids` needs, and nothing else.
  pub fn evaluation_snapshot_for(&self, ids: impl IntoIterator<Item = Id>) -> EvaluationSnapshot {
    let mut small = StateManager::new(StyleXOptions::default());

    small.options = self.options.clone();

    small.input_source_file = self.input_source_file.clone();
    small.module_source = self.module_source.clone();

    small.binding_reassignments = Rc::clone(&self.binding_reassignments);
    small.binding_mutations = Rc::clone(&self.binding_mutations);
    small.binding_deep_mutations = Rc::clone(&self.binding_deep_mutations);
    small.declared_bindings = Rc::clone(&self.declared_bindings);

    small.call_positions = Rc::clone(&self.call_positions);

    small.imports = self.imports.clone();
    small.export_id = self.export_id.clone();
    small.cycle = self.cycle;

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
        reached.remove(&id);
        omitted.push(id);
        omitted.append(&mut queue);
        break;
      }

      let ident = Ident::new(id.0.clone(), DUMMY_SP, id.1);

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

        // An initializer whose references outran the budget leaves names nobody followed, which is the same terminal answer as outrunning the binding bound: the snapshot may not be folded against.
        if !complete {
          omitted.push(id);
          omitted.append(&mut queue);
          break;
        }
      }

      if let Some((import, _specifier)) = self.import_binding(&ident) {
        // Keyed by span: one import binding several names is carried once.
        if carried_import_spans.insert(import.span) {
          small.push_top_import(import.clone());
        }
      }
    }

    small.adopt_module_root_bindings(root_bindings);

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

    small.declarations_state.opaque_faults = self
      .declarations_state
      .opaque_faults
      .iter()
      .filter(|(id, _)| reached.contains(*id))
      .map(|(id, fault)| (id.clone(), *fault))
      .collect();

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
  /// Whether every reference and every level was read.
  complete: bool,
}

impl Referenced<'_> {
  /// One level of the descent, refused at the ceiling.
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

  /// A dotted property is a spelling, not a reference: `a.b` reads `a`.
  fn visit_member_expr(&mut self, member: &MemberExpr) {
    member.obj.visit_with(self);

    if let MemberProp::Computed(key) = &member.prop {
      key.expr.visit_with(self);
    }
  }

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

#[derive(Clone, Debug)]
pub enum EvaluationEffect {
  /// A nested producer's rule filed for the owning transform.
  Rule {
    key: RuleKey,
    style: Rc<InjectableStyleKind>,
  },
  /// The filing itself, which the evaluator memo reads to decline to remember a fold that produced a rule.
  NestedRuleFiled,
  /// A theme side-effect import.
  ThemeImport { at: Option<Span>, item: ModuleItem },
}

/// An ordered, owned record of what one or more invocations did.
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
  pub fn harvest(spawned: &mut StateManager) -> Self {
    Self::harvest_past(spawned, None)
  }

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

        slot => spawned.pending_module_items.push(PendingInsertion {
          slot,
          item: pending.item,
        }),
      }
    }

    journal
  }

  /// Replays everything, in order, through the registration paths the module already has.
  pub fn replay_into(self, state: &mut StateManager) {
    for effect in self.entries {
      match effect {
        EvaluationEffect::Rule { key, style } => state.file_evaluation_rule(key, style),
        EvaluationEffect::NestedRuleFiled => state.note_evaluation_rule_filed(),

        EvaluationEffect::ThemeImport { at: None, item } => {
          state.queue_theme_import_if_absent(item);
        },

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
            // A missing source must still retain its side-effect import.
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
  /// The journal an invocation on this thread is publishing into, if one is installed.
  static COLLECTING: RefCell<Option<Rc<RefCell<EvaluationEffects>>>> =
    const { RefCell::new(None) };
}

/// The journal installed for the current scope, for as long as it is held.
pub struct ScopedEffects {
  journal: Rc<RefCell<EvaluationEffects>>,
  previous: Option<Rc<RefCell<EvaluationEffects>>>,
}

impl ScopedEffects {
  /// Installs a fresh journal for this thread and answers the handle that owns it.
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
  pub fn absorb(&self, effects: EvaluationEffects) {
    self.journal.borrow_mut().absorb(effects);
  }

  /// Uninstalls the sink and answers what was collected.
  pub fn finish(self) -> EvaluationEffects {
    std::mem::take(&mut *self.journal.borrow_mut())
  }

  /// Uninstalls the sink and throws the journal away.
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

/// Records `effect` against the installed sink, answering whether there was one.
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

/// Test whether this thread is collecting invocation effects.
pub fn effects_are_collected() -> bool {
  COLLECTING.with(|slot| slot.borrow().is_some())
}

#[cfg(test)]
#[path = "tests/evaluation_scope_test.rs"]
mod evaluation_scope_test;

// Holding the pair means a re-file of a genuinely different style is published again, as it must be, and a re-file of the same one is not.
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
  /// The publication ledger the compiler callback running on this thread is marking against, if one is installed.
  /// A collector with no ledger is the ordinary case -- an effect written straight onto the caller's own state, journalled at the end -- and must keep behaving exactly as it did.
  static PUBLISHING: RefCell<Option<Rc<RefCell<PublicationLedger>>>> =
    const { RefCell::new(None) };
}

/// The publication scope one compiler callback runs inside.
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

  /// What `spawned` is still holding that the collector has not already been given.
  pub fn harvest(&self, spawned: &mut StateManager) -> EvaluationEffects {
    // Whatever an immutable host operation queued and no writer has let out yet.
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

/// Whether a compiler callback on this thread is publishing its effects as it produces them.
pub fn effects_are_published() -> bool {
  PUBLISHING.with(|slot| slot.borrow().is_some())
}

/// Hands one effect to the collector and marks it on the ledger, where both are open.
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
  /// Files one rule the evaluation produced, and publishes it where a callback is collecting.
  /// The local map is the cache this state answers its own reads from and is written whether or not anybody is collecting -- a producer that reads back what it filed must find it.
  pub fn file_evaluation_rule(&mut self, key: RuleKey, style: Rc<InjectableStyleKind>) {
    // Anything an immutable host operation queued happened *before* this write, so it is published before it.
    self.replay_queued_evaluation_effects();

    self
      .other_injected_css_rules
      .insert(key.clone(), Rc::clone(&style));

    let marked = key.clone();
    let identity = style_identity(&style);

    let _published =
      publish_against_the_ledger(EvaluationEffect::Rule { key, style }, move |ledger| {
        ledger.mark_rule(marked, identity)
      });
  }

  /// Records that one nested rule was filed, and publishes the filing where a callback is collecting.
  pub fn note_evaluation_rule_filed(&mut self) {
    self.replay_queued_evaluation_effects();
    self.note_nested_rule_filed();

    let _published = publish_against_the_ledger(
      EvaluationEffect::NestedRuleFiled,
      PublicationLedger::mark_filing,
    );
  }
}

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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UnreplayableEffects {
  /// Items queued for a slot that is not a theme import -- a producer's own placement.
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

/// What `spawned` is holding that no journal can carry, or `None` where it is holding none of it.
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

// ============ effects an immutable host operation could not write ============ The smallest queue any heap offers.
pub const MIN_PENDING_EFFECTS: usize = 4096;

/// The most any queue will hold, however large the module.
pub const MAX_PENDING_EFFECTS_LIMIT: usize = 65_536;

/// Four events per folded entry.
const EFFECTS_PER_ENTRY: usize = 4;

/// Two events per source byte.
const EFFECTS_PER_SOURCE_BYTE: usize = 2;

impl EvaluationHeap {
  /// Appends `effects` to the queue, or refuses them for want of room.
  fn queue_effects(&mut self, effects: EvaluationEffects, limit: usize) {
    let incoming = effects.len();

    if self.pending.len().saturating_add(incoming) > limit {
      self.pending_overflowed = true;

      return;
    }

    self.pending.extend(effects.into_entries());
  }

  /// Queues against the floor, for a heap with no state to size against.
  pub fn queue_effects_at_the_floor(&mut self, effects: EvaluationEffects) {
    self.queue_effects(effects, MIN_PENDING_EFFECTS);
  }

  /// Emptying is what makes a flush safe to re-enter: the writers flush before they write, and a replay goes back through those writers, so the second flush must find nothing.
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
  pub fn replay_queued_evaluation_effects(&mut self) {
    let heap = Rc::clone(&self.evaluation_heap);

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
  pub fn pending_effects_overflowed(&self) -> bool {
    self.evaluation_heap.borrow().pending_overflowed()
  }
}

/// Publishes one completed callback batch into the active owner without ever borrowing that owner's StateManager.
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
