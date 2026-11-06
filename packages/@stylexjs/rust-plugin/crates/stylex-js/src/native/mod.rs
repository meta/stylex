//! Bounded modeled JavaScript values and pure builtins; never source evaluation.
use indexmap::IndexMap;
use std::{
  any::Any,
  cell::{Cell, RefCell},
  rc::{Rc, Weak},
};
use swc_core::{common::Span, ecma::ast::Id};
#[cfg(test)]
#[path = "quota_refusal_tests.rs"]
mod quota_refusal_tests;

#[cfg(test)]
#[path = "map_result_quota_tests.rs"]
mod map_result_quota_tests;

mod builtins;
mod decimal;
mod integer;
mod string_replace;
mod value;
use value::*;
pub use value::{binary, integer, is_truthy, number_string, string_units, to_int32, wtf_units};
pub const MAX_DEPTH: usize = 32;
pub const ABSOLUTE_MAX_DEPTH: usize = 256;
pub const MAX_FUEL: usize = 50_000;
pub const MAX_OUTPUT: usize = 262_144;
#[derive(Clone)]
pub struct FunctionRef {
  pub identity: usize,
  pub value: Rc<dyn Any>,
}
impl std::fmt::Debug for FunctionRef {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_tuple("FunctionRef").field(&self.identity).finish()
  }
}
impl PartialEq for FunctionRef {
  fn eq(&self, other: &Self) -> bool {
    self.identity == other.identity
  }
}
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
  Undefined,
  /// Internal absent array slot; Get/iteration expose undefined.
  Hole,
  Null,
  Bool(bool),
  Number(f64),
  String(String),
  Utf16(Rc<Vec<u16>>),
  Array(Rc<Vec<Value>>),
  Object(Rc<IndexMap<String, Value>>),
  NullObject(Rc<IndexMap<String, Value>>),
  Function(FunctionRef),
  OpaqueObject(u64),
  OpaqueFunction(u64),
  OpaquePrimitive(u64),
}
/// The actual quota guard that refused, distinct from type/model/user errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuotaKind {
  Characters,
  /// Size requested by an actual string amplifier before allocating it.
  DeclaredCharacters {
    count: usize,
  },
  /// Valid JavaScript array length requested before allocating its slots.
  DeclaredArrayEntries,
  ArrayEntries,
  ObjectEntries,
  Output,
  Fuel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuotaRefusal {
  pub kind: QuotaKind,
  pub limit: usize,
  pub observed: usize,
}

pub struct CounterLedger {
  fuel: Cell<usize>,
  depth: Cell<usize>,
  value_depth: Cell<usize>,
  result_depth: Cell<Option<usize>>,
  depth_limit_refusal: Cell<Option<usize>>,
  output: Cell<usize>,
  input: Cell<usize>,
  output_capacity: Cell<usize>,
  exhausted: Cell<bool>,
  quota_refusal: Cell<Option<QuotaRefusal>>,
  host_epoch: Cell<u64>,
  model_refusal_epoch: Cell<u64>,
  execution_position: Cell<Option<u32>>,
  read_observer: RefCell<Option<Rc<dyn Any>>>,
  result_limits: Cell<Option<(usize, usize)>>,
}
type BindingValues = RefCell<IndexMap<Id, Rc<dyn Any>>>;
pub struct Budget {
  counters: Rc<CounterLedger>,
  binding_values: Rc<BindingValues>,
}
impl std::ops::Deref for Budget {
  type Target = CounterLedger;
  fn deref(&self) -> &CounterLedger {
    &self.counters
  }
}
impl Default for Budget {
  fn default() -> Self {
    Self {
      counters: Rc::new(CounterLedger {
        fuel: Cell::new(MAX_FUEL),
        depth: Cell::new(0),
        value_depth: Cell::new(0),
        result_depth: Cell::new(None),
        depth_limit_refusal: Cell::new(None),
        output: Cell::new(MAX_OUTPUT),
        input: Cell::new(0),
        output_capacity: Cell::new(MAX_OUTPUT),
        exhausted: Cell::new(false),
        quota_refusal: Cell::new(None),
        host_epoch: Cell::new(0),
        model_refusal_epoch: Cell::new(0),
        execution_position: Cell::new(None),
        read_observer: RefCell::new(None),
        result_limits: Cell::new(None),
      }),
      binding_values: Rc::default(),
    }
  }
}
impl Budget {
  fn record_quota_refusal(&self, refusal: QuotaRefusal) {
    if self.quota_refusal.get().is_none() {
      self.quota_refusal.set(Some(refusal));
    }
    self.exhausted.set(true);
  }

  fn spend(&self, n: usize) -> Option<()> {
    if self.exhausted.get() {
      return None;
    }
    match self.fuel.get().checked_sub(n) {
      Some(left) => self.fuel.set(left),
      None => {
        let limit = MAX_FUEL.max(self.input.get().saturating_mul(4));
        self.record_quota_refusal(QuotaRefusal {
          kind: QuotaKind::Fuel,
          limit,
          observed: limit.saturating_sub(self.fuel.get()).saturating_add(n),
        });
        return None;
      },
    }
    Some(())
  }
  fn allocate(&self, n: usize) -> Option<()> {
    if self.exhausted.get() {
      return None;
    }
    match self.output.get().checked_sub(n) {
      Some(left) => self.output.set(left),
      None => {
        let limit = self.output_capacity.get();
        self.record_quota_refusal(QuotaRefusal {
          kind: QuotaKind::Output,
          limit,
          observed: limit.saturating_sub(self.output.get()).saturating_add(n),
        });
        return None;
      },
    }
    Some(())
  }
  fn scale_input(&self, bytes: usize) {
    let old = self.input.get();
    if bytes <= old {
      return;
    }
    self.input.set(bytes);
    let fuel = MAX_FUEL
      .max(bytes.saturating_mul(4))
      .saturating_sub(MAX_FUEL.max(old.saturating_mul(4)));
    let output = MAX_OUTPUT
      .max(bytes.saturating_mul(16))
      .saturating_sub(MAX_OUTPUT.max(old.saturating_mul(16)));
    self.fuel.set(self.fuel.get().saturating_add(fuel));
    self.output.set(self.output.get().saturating_add(output));
    self
      .output_capacity
      .set(self.output_capacity.get().saturating_add(output));
  }
}
thread_local! { static ACTIVE: RefCell<Option<Rc<Budget>>> = const { RefCell::new(None) }; }
struct Scope {
  previous: Option<Rc<Budget>>,
  owns: bool,
}
impl Drop for Scope {
  fn drop(&mut self) {
    if self.owns {
      ACTIVE.with(|s| *s.borrow_mut() = self.previous.take());
    }
  }
}
pub fn with_evaluation<T>(operation: impl FnOnce() -> T) -> T {
  let owns = ACTIVE.with(|s| {
    let mut slot = s.borrow_mut();
    if slot.is_some() {
      false
    } else {
      *slot = Some(Rc::default());
      true
    }
  });
  let _scope = Scope {
    previous: None,
    owns,
  };
  operation()
}
pub fn with_evaluation_sized<T>(bytes: usize, operation: impl FnOnce() -> T) -> T {
  with_evaluation(|| {
    ACTIVE.with(|s| {
      if let Some(b) = s.borrow().as_ref() {
        b.scale_input(bytes);
      }
    });
    operation()
  })
}
pub fn with_isolated_evaluation<T>(operation: impl FnOnce() -> T) -> T {
  let _scope = Scope {
    previous: ACTIVE.with(|s| s.borrow_mut().take()),
    owns: true,
  };
  operation()
}
pub fn with_isolated_budget<T>(operation: impl FnOnce() -> T) -> T {
  with_isolated_evaluation(operation)
}
#[derive(Clone)]
pub struct BudgetSnapshot(Rc<Budget>);
#[derive(Clone)]
pub struct BudgetLease {
  counters: Rc<CounterLedger>,
  bindings: Weak<BindingValues>,
}
impl BudgetLease {
  pub fn upgrade(&self) -> Option<BudgetSnapshot> {
    Some(BudgetSnapshot(Rc::new(Budget {
      counters: self.counters.clone(),
      binding_values: self.bindings.upgrade().unwrap_or_default(),
    })))
  }
}
pub fn capture_budget() -> Option<BudgetSnapshot> {
  ACTIVE.with(|s| s.borrow().as_ref().cloned().map(BudgetSnapshot))
}
pub fn capture_budget_lease() -> Option<BudgetLease> {
  ACTIVE.with(|s| {
    s.borrow().as_ref().map(|b| BudgetLease {
      counters: b.counters.clone(),
      bindings: Rc::downgrade(&b.binding_values),
    })
  })
}
pub fn with_captured_evaluation<T>(snapshot: &BudgetSnapshot, operation: impl FnOnce() -> T) -> T {
  let previous = ACTIVE.with(|s| s.borrow_mut().replace(snapshot.0.clone()));
  let _scope = Scope {
    previous,
    owns: true,
  };
  operation()
}
pub fn has_active_evaluation() -> bool {
  ACTIVE.with(|s| s.borrow().is_some())
}
pub fn output_limit() -> usize {
  ACTIVE.with(|s| {
    s.borrow()
      .as_ref()
      .map_or(MAX_OUTPUT, |b| b.output_capacity.get())
  })
}
pub fn budget_exhausted() -> bool {
  ACTIVE.with(|s| s.borrow().as_ref().is_some_and(|b| b.exhausted.get()))
}
/// Reads structured quota evidence from the current operation, never from text.
pub fn quota_refusal() -> Option<QuotaRefusal> {
  ACTIVE.with(|slot| {
    slot
      .borrow()
      .as_ref()
      .and_then(|budget| budget.quota_refusal.get())
  })
}

/// Records only evidence from a quota guard that actually failed.
pub fn refuse_quota<T>(kind: QuotaKind, limit: usize, observed: usize) -> Option<T> {
  ACTIVE.with(|slot| {
    if let Some(budget) = slot.borrow().as_ref() {
      budget.record_quota_refusal(QuotaRefusal {
        kind,
        limit,
        observed,
      });
    }
  });
  None
}

pub fn refuse_resource<T>() -> Option<T> {
  ACTIVE.with(|s| {
    if let Some(b) = s.borrow().as_ref() {
      b.exhausted.set(true);
    }
  });
  None
}
pub fn charge_work(n: usize) -> Option<()> {
  ACTIVE.with(|s| s.borrow().as_ref().map_or(Some(()), |b| b.spend(n)))
}
pub fn charge_output(n: usize) -> Option<()> {
  ACTIVE.with(|s| s.borrow().as_ref().map_or(Some(()), |b| b.allocate(n)))
}
pub fn host_epoch() -> u64 {
  ACTIVE.with(|s| s.borrow().as_ref().map_or(0, |b| b.host_epoch.get()))
}
thread_local! {
  // Evaluator states/AST owners are Rc-based and stay on their compiler thread.
  static MEMO_HOST_REVISION: Cell<u64> = const { Cell::new(0) };
}

/// Actual host operations invalidate memo entries even across budget scopes.
pub fn memo_host_revision() -> u64 {
  MEMO_HOST_REVISION.with(Cell::get)
}

pub fn observe_host_operation() {
  MEMO_HOST_REVISION.with(|revision| revision.set(revision.get().wrapping_add(1)));
  ACTIVE.with(|s| {
    if let Some(b) = s.borrow().as_ref() {
      b.host_epoch.set(b.host_epoch.get().saturating_add(1));
    }
  });
}
pub fn binding_value<T: 'static>(id: &Id) -> Option<Rc<T>> {
  ACTIVE.with(|s| {
    s.borrow()
      .as_ref()
      .and_then(|b| b.binding_values.borrow().get(id).cloned())
      .and_then(|v| v.downcast().ok())
  })
}
pub fn remember_binding<T: 'static>(id: Id, value: Rc<T>) {
  ACTIVE.with(|s| {
    if let Some(b) = s.borrow().as_ref() {
      b.binding_values.borrow_mut().entry(id).or_insert(value);
    }
  });
}
pub fn execution_position() -> Option<u32> {
  ACTIVE.with(|s| s.borrow().as_ref().and_then(|b| b.execution_position.get()))
}
struct PositionScope {
  budget: Rc<Budget>,
  previous: Option<u32>,
}
impl Drop for PositionScope {
  fn drop(&mut self) {
    self.budget.execution_position.set(self.previous);
  }
}
pub fn with_execution_position<T>(position: Option<u32>, operation: impl FnOnce() -> T) -> T {
  with_evaluation(|| {
    let b = ACTIVE.with(|s| s.borrow().as_ref().cloned());
    let _scope = b.map(|budget| {
      let previous = budget.execution_position.replace(position);
      PositionScope { budget, previous }
    });
    operation()
  })
}
pub fn with_callsite<T>(position: u32, operation: impl FnOnce() -> T) -> T {
  with_execution_position(
    execution_position().or((position != 0).then_some(position)),
    operation,
  )
}
pub struct DepthGuard {
  budget: Rc<Budget>,
}
impl Drop for DepthGuard {
  fn drop(&mut self) {
    self
      .budget
      .depth
      .set(self.budget.depth.get().saturating_sub(1));
  }
}
fn enter_depth(budget: &Rc<Budget>) -> Option<DepthGuard> {
  if budget.depth.get() >= MAX_DEPTH {
    return refuse_resource();
  }
  budget.spend(1)?;
  budget.depth.set(budget.depth.get() + 1);
  Some(DepthGuard {
    budget: budget.clone(),
  })
}
pub fn enter_evaluation_depth() -> Option<DepthGuard> {
  ACTIVE
    .with(|s| s.borrow().as_ref().cloned())
    .and_then(|b| enter_depth(&b))
}
pub trait Callbacks {
  fn call(&mut self, function: &FunctionRef, args: Vec<Value>) -> Option<Value>;
  fn static_builtin(&mut self, _namespace: &str, _method: &str, _args: &[Value]) -> Option<Value> {
    None
  }
  fn is_array(&mut self, _value: &Value) -> Option<bool> {
    None
  }
}
pub fn constant(namespace: &str, key: &str) -> Option<Value> {
  builtins::constant(namespace, key)
}
struct Context<'a> {
  budget: Rc<Budget>,
  callbacks: &'a mut dyn Callbacks,
  callsite: Option<Span>,
}
impl Context<'_> {
  fn call_value(&mut self, function: &Value, args: Vec<Value>, _span: Span) -> Option<Value> {
    let _depth = enter_depth(&self.budget)?;
    let value = match function {
      Value::Function(function) => self.callbacks.call(function, args)?,
      _ => return None,
    };
    // Account before the array method retains the callback output. A callback
    // returning a large value per element cannot build an unmetered result.
    enforce_result_limits(&value)?;
    self.budget.allocate(value_size(&value))?;
    Some(value)
  }
  fn same_value(&mut self, a: &Value, b: &Value) -> Option<bool> {
    if let (Value::Number(a), Value::Number(b)) = (a, b) {
      Some(a.to_bits() == b.to_bits() || a.is_nan() && b.is_nan())
    } else {
      strict_equal(a, b)
    }
  }
  fn truthy(&mut self, v: &Value) -> Option<bool> {
    Some(is_truthy(v))
  }
  fn iterable(&mut self, v: &Value) -> Option<Vec<Value>> {
    iterable(v)
  }
  fn member_value(&mut self, v: &Value, key: &str) -> Option<Value> {
    member_value(v, key)
  }
  fn enumerable_entries(&mut self, v: &Value) -> Option<IndexMap<String, Value>> {
    own_entries(v)
  }
  fn units(&mut self, v: &Value, _span: Span, _depth: usize) -> Option<Vec<u16>> {
    v.to_units()
  }
  fn enter(&self) -> Option<DepthGuard> {
    enter_depth(&self.budget)
  }
}
pub fn apply(
  namespace: Option<&str>,
  method: &str,
  receiver: Option<&Value>,
  args: &[Value],
  callbacks: &mut dyn Callbacks,
) -> Option<Value> {
  with_evaluation(|| {
    let budget = ACTIVE
      .with(|s| s.borrow().as_ref().cloned())
      .unwrap_or_default();
    budget.spend(1)?;
    let mut context = Context {
      budget,
      callbacks,
      callsite: None,
    };
    let value = match (namespace, receiver) {
      (Some(ns), _) => builtins::static_method(ns, method, args, &mut context),
      (None, Some(r)) => builtins::method(r, method, args, &mut context),
      (None, None) => builtins::global(method, args, &mut context),
    }?;
    enforce_result_limits(&value)?;
    context.budget.allocate(value_size(&value))?;
    Some(value)
  })
}

pub fn supports_static(namespace: &str, method: &str) -> bool {
  match namespace {
    "Math" => builtins::math_supported(method),
    "String" => matches!(method, "fromCharCode" | "fromCodePoint" | "raw"),
    "Number" => matches!(
      method,
      "isFinite" | "isNaN" | "isInteger" | "isSafeInteger" | "parseInt" | "parseFloat"
    ),
    "Array" => matches!(method, "isArray" | "from" | "of"),
    "Object" => matches!(
      method,
      "keys"
        | "values"
        | "entries"
        | "getOwnPropertyNames"
        | "getOwnPropertySymbols"
        | "hasOwn"
        | "fromEntries"
        | "groupBy"
        | "is"
        | "isExtensible"
        | "isFrozen"
        | "isSealed"
    ),
    _ => false,
  }
}
pub fn supports_method(receiver: &Value, method: &str) -> bool {
  match receiver {
    Value::Array(_) => matches!(
      method,
      "map"
        | "filter"
        | "flatMap"
        | "forEach"
        | "every"
        | "some"
        | "find"
        | "findIndex"
        | "findLast"
        | "findLastIndex"
        | "reduce"
        | "reduceRight"
        | "slice"
        | "at"
        | "includes"
        | "indexOf"
        | "lastIndexOf"
        | "concat"
        | "join"
        | "flat"
        | "toReversed"
        | "toSorted"
        | "toSpliced"
        | "with"
        | "toString"
    ),
    Value::String(_) | Value::Utf16(_) => matches!(
      method,
      "at"
        | "charAt"
        | "charCodeAt"
        | "codePointAt"
        | "concat"
        | "includes"
        | "indexOf"
        | "lastIndexOf"
        | "slice"
        | "substring"
        | "substr"
        | "startsWith"
        | "endsWith"
        | "repeat"
        | "replace"
        | "replaceAll"
        | "split"
        | "trim"
        | "trimStart"
        | "trimEnd"
        | "trimLeft"
        | "trimRight"
        | "toLowerCase"
        | "toUpperCase"
        | "padStart"
        | "padEnd"
        | "isWellFormed"
        | "toWellFormed"
        | "normalize"
        | "toString"
        | "valueOf"
    ),
    Value::Number(_) => matches!(
      method,
      "toString" | "valueOf" | "toFixed" | "toPrecision" | "toExponential"
    ),
    Value::Bool(_) => matches!(method, "toString" | "valueOf"),
    Value::Object(properties) => {
      properties
        .get(method)
        .is_some_and(|value| matches!(value, Value::Function(_)))
        || matches!(
          method,
          "toString" | "valueOf" | "hasOwnProperty" | "propertyIsEnumerable" | "isPrototypeOf"
        )
    },
    Value::NullObject(properties) => properties
      .get(method)
      .is_some_and(|value| matches!(value, Value::Function(_))),
    _ => false,
  }
}

struct ReadObserverScope {
  ledger: Rc<CounterLedger>,
  previous: Option<Rc<dyn Any>>,
}
impl Drop for ReadObserverScope {
  fn drop(&mut self) {
    *self.ledger.read_observer.borrow_mut() = self.previous.take();
  }
}
pub fn with_scoped_read_observer<T>(observer: Rc<dyn Any>, operation: impl FnOnce() -> T) -> T {
  with_evaluation(|| {
    let ledger = ACTIVE.with(|slot| slot.borrow().as_ref().map(|budget| budget.counters.clone()));
    let _scope = ledger.map(|ledger| {
      let previous = ledger.read_observer.borrow_mut().replace(observer);
      ReadObserverScope { ledger, previous }
    });
    operation()
  })
}
pub fn scoped_read_observer<T: 'static>() -> Option<Rc<T>> {
  ACTIVE.with(|slot| {
    slot
      .borrow()
      .as_ref()
      .and_then(|budget| budget.read_observer.borrow().clone())
      .and_then(|observer| observer.downcast().ok())
  })
}

/// Configured per-result ceilings remain distinct from cumulative operation
/// work/allocation counters. Installing them cannot replenish spent work.
pub fn set_result_limits(characters: usize, entries: usize) {
  ACTIVE.with(|slot| {
    if let Some(budget) = slot.borrow().as_ref() {
      let limits = budget
        .result_limits
        .get()
        .map_or((characters, entries), |(c, e)| {
          (c.min(characters), e.min(entries))
        });
      if budget.result_limits.get().is_none() {
        let capacity = MAX_OUTPUT.max(
          limits
            .0
            .saturating_mul(4)
            .saturating_add(limits.1.saturating_mul(40)),
        );
        let delta = capacity.saturating_sub(budget.output_capacity.get());
        budget.output.set(budget.output.get().saturating_add(delta));
        budget
          .output_capacity
          .set(budget.output_capacity.get().saturating_add(delta));
      }
      budget.result_limits.set(Some(limits));
    }
  })
}
pub fn character_limit() -> usize {
  ACTIVE.with(|slot| {
    slot
      .borrow()
      .as_ref()
      .and_then(|budget| budget.result_limits.get())
      .map_or(output_limit(), |limits| limits.0)
  })
}
pub fn entry_limit() -> usize {
  ACTIVE.with(|slot| {
    slot
      .borrow()
      .as_ref()
      .and_then(|budget| budget.result_limits.get())
      .map_or(output_limit(), |limits| limits.1)
  })
}
/// Count nested containers as they become successful callback results. The
/// output array's own known slots are included by the caller. This is a guard
/// on the actual result prefix, never a scan of callback code or future reads.
fn check_array_result_entries(value: &Value, mut entries: usize) -> Option<usize> {
  if !matches!(
    value,
    Value::Array(_) | Value::Object(_) | Value::NullObject(_)
  ) {
    return Some(entries);
  }
  let limit = entry_limit();
  let mut pending = vec![value];
  while let Some(value) = pending.pop() {
    let children = match value {
      Value::Array(values) => {
        entries = entries.saturating_add(values.len());
        if entries > limit {
          return refuse_quota(QuotaKind::ArrayEntries, limit, entries);
        }
        values.as_slice()
      },
      Value::Object(values) | Value::NullObject(values) => {
        entries = entries.saturating_add(values.len());
        if entries > limit {
          return refuse_quota(QuotaKind::ArrayEntries, limit, entries);
        }
        for value in values.values() {
          if matches!(
            value,
            Value::Array(_) | Value::Object(_) | Value::NullObject(_)
          ) {
            pending.push(value);
          }
        }
        continue;
      },
      _ => continue,
    };
    charge_work(1)?;
    for child in children {
      if matches!(
        child,
        Value::Array(_) | Value::Object(_) | Value::NullObject(_)
      ) {
        pending.push(child);
      }
    }
  }
  Some(entries)
}

pub fn enforce_result_limits(value: &Value) -> Option<()> {
  let mut work = vec![(value, 0usize)];
  let mut chars = 0usize;
  let mut entries = 0usize;
  let char_limit = character_limit();
  let entries_limit = entry_limit();
  while let Some((value, depth)) = work.pop() {
    charge_work(1)?;
    match value {
      Value::String(text) => chars = chars.checked_add(text.encode_utf16().count())?,
      Value::Utf16(text) => chars = chars.checked_add(text.len())?,
      Value::Array(values) => {
        if depth >= result_depth_limit() {
          return refuse_result_depth();
        }
        entries = entries.checked_add(values.len())?;
        if entries > entries_limit {
          return refuse_quota(QuotaKind::ArrayEntries, entries_limit, entries);
        }
        for value in values.iter().rev() {
          work.push((value, depth + 1));
        }
      },
      Value::Object(values) | Value::NullObject(values) => {
        if depth >= result_depth_limit() {
          return refuse_result_depth();
        }
        entries = entries.checked_add(values.len())?;
        if entries > entries_limit {
          return refuse_quota(QuotaKind::ObjectEntries, entries_limit, entries);
        }
        for (key, value) in values.iter().rev() {
          chars = chars.checked_add(key.encode_utf16().count())?;
          work.push((value, depth + 1));
        }
      },
      _ => {},
    }
    if chars > char_limit {
      return refuse_quota(QuotaKind::Characters, char_limit, chars);
    }
  }
  Some(())
}

/// Capability refusals are not author exceptions. The epoch also prevents a
/// cached refusal from losing its classification in a later callback frame.
pub fn model_refusal_epoch() -> u64 {
  ACTIVE.with(|slot| {
    slot
      .borrow()
      .as_ref()
      .map_or(0, |budget| budget.model_refusal_epoch.get())
  })
}
pub fn observe_model_refusal() {
  ACTIVE.with(|slot| {
    if let Some(budget) = slot.borrow().as_ref() {
      budget
        .model_refusal_epoch
        .set(budget.model_refusal_epoch.get().wrapping_add(1));
    }
  })
}

/// Per-result data depth is distinct from runtime call nesting and spent work.
/// Re-installing a ceiling cannot reset the ledger or relax an existing root.
pub fn set_result_depth_limit(limit: usize) {
  let limit = limit.clamp(1, ABSOLUTE_MAX_DEPTH);
  ACTIVE.with(|slot| {
    if let Some(budget) = slot.borrow().as_ref() {
      budget.result_depth.set(Some(
        budget
          .result_depth
          .get()
          .map_or(limit, |old| old.min(limit)),
      ));
    }
  })
}
pub fn result_depth_limit() -> usize {
  ACTIVE.with(|slot| {
    slot
      .borrow()
      .as_ref()
      .and_then(|budget| budget.result_depth.get())
      .unwrap_or(MAX_DEPTH)
  })
}
pub fn result_depth_refusal() -> Option<usize> {
  ACTIVE.with(|slot| {
    slot
      .borrow()
      .as_ref()
      .and_then(|budget| budget.depth_limit_refusal.get())
  })
}
pub fn refuse_result_depth<T>() -> Option<T> {
  let limit = result_depth_limit();
  ACTIVE.with(|slot| {
    if let Some(budget) = slot.borrow().as_ref() {
      budget.depth_limit_refusal.set(Some(limit));
      budget.exhausted.set(true);
    }
  });
  None
}
pub struct ValueDepthGuard {
  budget: Rc<Budget>,
}
impl Drop for ValueDepthGuard {
  fn drop(&mut self) {
    self
      .budget
      .value_depth
      .set(self.budget.value_depth.get().saturating_sub(1));
  }
}
pub fn enter_value_depth() -> Option<ValueDepthGuard> {
  let budget = ACTIVE.with(|slot| slot.borrow().as_ref().cloned())?;
  if budget.value_depth.get() >= result_depth_limit() {
    return refuse_result_depth();
  }
  budget.spend(1)?;
  budget.value_depth.set(budget.value_depth.get() + 1);
  Some(ValueDepthGuard { budget })
}
