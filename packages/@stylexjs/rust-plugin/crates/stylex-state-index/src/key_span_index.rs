use std::{cell::OnceCell, cmp::Reverse, rc::Rc};

use rustc_hash::{FxHashMap, FxHashSet};
use swc_core::{
  atoms::Atom,
  common::{BytePos, DUMMY_SP, EqIgnoreSpan, Span, Spanned},
  ecma::{
    ast::{CallExpr, Callee, Expr, Module, ObjectLit},
    visit::{Visit, VisitWith, noop_visit_type},
  },
};

use stylex_ast::ast::keys::{
  collect_object_lit_keys, namespace_name_from_prop_key, prop_as_key_value,
};
use stylex_utils::hash::stable_hash_wide;

/// A module-relative byte offset for same-file comparisons across source maps.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FileOffset(u32);

/// The module start used to normalize source-map positions; absence must not default to zero.
#[derive(Clone, Copy, Debug)]
pub struct ModuleBase(BytePos);

impl ModuleBase {
  pub fn of(module: &Module) -> Self {
    Self(module.span.lo)
  }
}

impl FileOffset {
  /// Computes a module-relative byte offset.
  /// Debug-asserts that the position is not before the base; release builds clamp to zero.
  pub(crate) fn of(position: BytePos, base: ModuleBase) -> Self {
    debug_assert!(
      position >= base.0,
      "a position at {:?} precedes the base of the module holding it, {:?}",
      position,
      base.0
    );

    Self::clamped(position, base)
  }

  /// Saturating offset arithmetic; production callers should use [`Self::of`] for its assertion.
  pub(crate) fn clamped(position: BytePos, base: ModuleBase) -> Self {
    Self(position.0.saturating_sub(base.0.0))
  }

  fn distance(self, other: Self) -> u32 {
    self.0.abs_diff(other.0)
  }

  /// Constructs a fixture offset directly.
  #[cfg(test)]
  pub(crate) fn at(offset: u32) -> Self {
    Self(offset)
  }
}

/// Indexes authored namespace-key spans in one module walk.
/// Candidate spans belong to the parsed module used to build the index.
#[derive(Clone, Debug)]
pub struct KeySpanIndex {
  by_key: FxHashMap<Atom, Vec<IndexedCandidate>>,
  /// Start of the indexed module in its source map.
  base: ModuleBase,
}

/// An authored key span with call and namespace signals for disambiguation.
#[derive(Clone, Debug)]
struct IndexedCandidate {
  span: Span,
  /// Value-object keys, empty for non-object values.
  namespace_value_keys: Vec<Atom>,
  /// Sibling keys shared by candidates from the same object argument.
  sibling_keys: Rc<Vec<Atom>>,
  candidate_offset: FileOffset,
  /// Callee shared by candidates from this object argument; ranks ahead of key overlap.
  callee: Rc<Callee>,
}

impl KeySpanIndex {
  pub fn build(module: &Module) -> Self {
    let mut index = Self {
      by_key: FxHashMap::default(),
      base: ModuleBase::of(module),
    };

    module.visit_with(&mut index);

    index
  }

  /// Resolves the authored key span, or [`DUMMY_SP`] for a missing or tied best candidate.
  pub fn resolve(&self, query: &NamespaceKeyQuery) -> Span {
    let candidates = match self.by_key.get(&Atom::from(query.namespace_key)) {
      Some(candidates) => candidates,
      None => return DUMMY_SP,
    };

    let mut best: Option<(CandidateRank, Span)> = None;
    let mut ambiguous = false;

    for candidate in candidates {
      let rank = candidate.rank(query);

      match best.as_ref() {
        None => {
          best = Some((rank, candidate.span));
          ambiguous = false;
        },
        Some((best_rank, _)) if rank > *best_rank => {
          best = Some((rank, candidate.span));
          ambiguous = false;
        },
        Some((best_rank, _)) if rank == *best_rank => {
          ambiguous = true;
        },
        Some(_) => {},
      }
    }

    if ambiguous {
      return DUMMY_SP;
    }

    best.map_or(DUMMY_SP, |(_, span)| span)
  }

  /// Records one candidate per key, using its last property and surviving value-object keys.
  fn index_object(&mut self, call: &CallExpr, object: &ObjectLit) {
    let sibling_keys = Rc::new(collect_object_lit_keys(object).collect::<Vec<_>>());

    if sibling_keys.is_empty() {
      return;
    }

    let candidate_lo = object_lo(object).unwrap_or(call.span.lo);
    let candidate_offset = FileOffset::of(candidate_lo, self.base);
    let callee = Rc::new(call.callee.clone());
    let mut in_this_object: FxHashMap<Atom, IndexedCandidate> = FxHashMap::default();

    for prop in &object.props {
      if let Some(key_value) = prop_as_key_value(prop)
        && let Some(name) = namespace_name_from_prop_key(&key_value.key)
      {
        in_this_object.insert(
          name,
          IndexedCandidate {
            span: key_value.key.span(),
            namespace_value_keys: object_lit_keys(&key_value.value).collect(),
            sibling_keys: Rc::clone(&sibling_keys),
            candidate_offset,
            callee: Rc::clone(&callee),
          },
        );
      }
    }

    // Each object contributes one candidate per name, so per-name lists retain source order.
    for (name, candidate) in in_this_object {
      self.by_key.entry(name).or_default().push(candidate);
    }
  }
}

impl Visit for KeySpanIndex {
  noop_visit_type!();

  fn visit_call_expr(&mut self, call: &CallExpr) {
    if let Some(object) = first_object_arg(call) {
      self.index_object(call, object);
    }

    call.visit_children_with(self);
  }
}

/// Candidate precedence follows field order: callee, value keys, siblings, then proximity.
/// Unmeasured distance is uniform across a query and cannot break its ties.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct CandidateRank {
  /// Callee agreement outranks overlap and proximity.
  pub(crate) callee_match: bool,
  pub(crate) namespace_value_overlap: usize,
  pub(crate) sibling_overlap: usize,
  pub(crate) distance_from_target: Reverse<Option<u32>>,
}

impl IndexedCandidate {
  fn rank(&self, query: &NamespaceKeyQuery) -> CandidateRank {
    CandidateRank {
      callee_match: query
        .callee
        .is_some_and(|callee| self.callee.as_ref().eq_ignore_span(callee)),
      namespace_value_overlap: overlap(&self.namespace_value_keys, &query.namespace_value_keys),
      sibling_overlap: overlap(&self.sibling_keys, &query.sibling_keys),
      distance_from_target: Reverse(
        query
          .target_offset
          .map(|target_offset| self.candidate_offset.distance(target_offset)),
      ),
    }
  }
}

/// Counts authored keys present in the compiled set, including repeated authored keys.
fn overlap(authored: &[Atom], compiled: &FxHashSet<Atom>) -> usize {
  authored
    .iter()
    .filter(|name| compiled.contains(*name))
    .count()
}

/// Compiled namespace and call signals used to rank authored candidates.
#[derive(Clone, Debug)]
pub struct NamespaceKeyQuery<'a> {
  pub namespace_key: &'a str,
  /// Keys shared by every namespace in the call.
  pub(crate) sibling_keys: Rc<FxHashSet<Atom>>,
  pub namespace_value_keys: FxHashSet<Atom>,
  /// Module-relative object position, if known.
  pub target_offset: Option<FileOffset>,
  /// Callee of the compiled call.
  pub(crate) callee: Option<&'a Callee>,
}

/// Shared lookup signals, cache digest, and lazily cloned expression for one compiled call.
pub struct CallLookup<'a> {
  call_expr: &'a CallExpr,
  /// Object argument resolved once for all namespace queries.
  object_arg: Option<&'a ObjectLit>,
  sibling_keys: Rc<FxHashSet<Atom>>,
  target_offset: Option<FileOffset>,
  digest: u128,
  /// Deep-cloned call expression, initialized only when a fallback or cache miss needs it.
  wrapped: OnceCell<Expr>,
}

impl<'a> CallLookup<'a> {
  /// Builds call signals using the base of the module holding `call_expr`.
  /// An absent base disables proximity rather than measuring from zero.
  pub fn new(call_expr: &'a CallExpr, module_base: Option<ModuleBase>) -> Self {
    let object_arg = first_object_arg(call_expr);
    let sibling_keys: Rc<FxHashSet<Atom>> = Rc::new(
      object_arg
        .map(|object| collect_object_lit_keys(object).collect())
        .unwrap_or_default(),
    );

    Self {
      call_expr,
      object_arg,
      digest: call_digest(call_expr, object_arg, &sibling_keys),
      sibling_keys,
      target_offset: module_base.and_then(|base| {
        object_arg
          .and_then(object_lo)
          .or_else(|| (!call_expr.span.is_dummy()).then_some(call_expr.span.lo))
          .map(|lo| FileOffset::of(lo, base))
      }),
      wrapped: OnceCell::new(),
    }
  }

  /// Returns the call half of the span cache key.
  pub fn digest(&self) -> u128 {
    self.digest
  }

  /// Returns the call expression, deep-cloned once on first use.
  pub fn wrapped(&self) -> &Expr {
    self
      .wrapped
      .get_or_init(|| Expr::Call(self.call_expr.clone()))
  }

  /// Builds a query for one namespace using the shared call signals.
  pub fn query(&self, namespace_key: &'a str) -> NamespaceKeyQuery<'a> {
    NamespaceKeyQuery {
      namespace_key,
      sibling_keys: Rc::clone(&self.sibling_keys),
      namespace_value_keys: self
        .object_arg
        .map(|object| namespace_value_keys(object, namespace_key))
        .unwrap_or_default(),
      target_offset: self.target_offset,
      callee: Some(&self.call_expr.callee),
    }
  }
}

/// Hashes the call identity and sibling keys for the span cache.
fn call_digest(
  call_expr: &CallExpr,
  object_arg: Option<&ObjectLit>,
  sibling_keys: &FxHashSet<Atom>,
) -> u128 {
  // Sibling-set iteration order is not part of the cache identity.
  let mut sorted_sibling_keys: Vec<&Atom> = sibling_keys.iter().collect();
  sorted_sibling_keys.sort();

  stable_hash_wide(&(
    "stylex-call-siblings:v1",
    &call_expr.callee,
    call_expr.span.lo.0,
    call_expr.span.hi.0,
    object_arg.map(|object| (object.span.lo.0, object.span.hi.0)),
    sorted_sibling_keys,
  ))
}

fn first_object_arg(call_expr: &CallExpr) -> Option<&ObjectLit> {
  match call_expr.args.first().map(|arg| arg.expr.as_ref()) {
    Some(Expr::Object(object)) => Some(object),
    _ => None,
  }
}

/// Returns the authored object position, or `None` for a dummy span.
fn object_lo(object: &ObjectLit) -> Option<BytePos> {
  (!object.span.is_dummy()).then_some(object.span.lo)
}

/// Reads the first matching namespace's value-object keys, or an empty set.
/// Compiled calls cannot repeat namespace names; authored candidates instead keep the last.
fn namespace_value_keys(object: &ObjectLit, namespace_key: &str) -> FxHashSet<Atom> {
  // A first matching non-object value must not fall through to a later duplicate.
  object
    .props
    .iter()
    .find(|prop| {
      prop_as_key_value(prop).is_some_and(|key_value| {
        namespace_name_from_prop_key(&key_value.key)
          .is_some_and(|name| name.as_ref() == namespace_key)
      })
    })
    .and_then(prop_as_key_value)
    .and_then(|key_value| match key_value.value.as_ref() {
      Expr::Object(value) => Some(collect_object_lit_keys(value).collect()),
      _ => None,
    })
    .unwrap_or_default()
}

fn object_lit_keys(value: &Expr) -> impl Iterator<Item = Atom> + '_ {
  let object = match value {
    Expr::Object(object) => Some(object),
    _ => None,
  };

  object.into_iter().flat_map(collect_object_lit_keys)
}
