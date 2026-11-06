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

/// How far into its own file a position sits.
///
/// The one thing the proximity tie-break may compare, and the reason it is a
/// type rather than a `u32`. Two `BytePos` in this compiler can name the same
/// character and hold different numbers: the key-span index is built from a
/// module re-parsed into the code frame's shared, process-global `SourceMap`,
/// while the call it places is read out of the per-transform one. A `SourceMap`
/// gives each file a start position after the previous file's end, so the two
/// agree only for the first file a process compiles -- and subtracting one from
/// the other silently ranked by "earliest in the file" from the second file on.
///
/// There is no way to build one of these except from a position *and* the
/// [`ModuleBase`] it belongs to, and no way to read the number back out. So the
/// subtraction cannot be forgotten at a new call site, and a raw `BytePos`
/// cannot reach [`FileOffset::distance`] at all.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FileOffset(u32);

/// Where a module starts, in the source map it was parsed into — what a
/// [`FileOffset`] is measured against.
///
/// **The canonical statement of why this is a type, which the rest of the
/// subsystem links to rather than repeats.** Two things fail if it is a bare
/// `BytePos`:
///
/// - Both arguments of [`FileOffset::of`] would be `BytePos`, so transposing
///   them compiles and answers zero for every candidate.
/// - A `BytePos` has a `Default`, and a base nobody recorded is `BytePos(0)` —
///   which makes the offset the raw position again. Silently, and only on the
///   configurations that never set one, which is how the bug this subsystem was
///   fixed for came back the first time it was fixed.
///
/// So there is no `Default`, no `From<BytePos>`, and one constructor that takes
/// the module itself. Where a base may be genuinely unavailable it is spelled
/// `Option<ModuleBase>`, and absent costs the proximity tie-break rather than
/// answering from zero.
#[derive(Clone, Copy, Debug)]
pub struct ModuleBase(BytePos);

impl ModuleBase {
  pub fn of(module: &Module) -> Self {
    Self(module.span.lo)
  }
}

impl FileOffset {
  /// Where `position` sits relative to `base`, the start of the module holding
  /// it.
  ///
  /// Saturating rather than panicking, because this is a diagnostic path and a
  /// build must not stop over a code frame. A position that precedes its own
  /// module's base is a caller error rather than a negative offset, though, so
  /// it is loud in a test build: clamping every candidate to zero is exactly
  /// the "rank by earliest in the file" failure this type exists to prevent.
  ///
  /// Crate-visible rather than private so the suite beside this module can
  /// reach it -- unconditionally, unlike [`Self::at`], because production calls
  /// it. The invariant is unchanged either way: a caller still has to supply a
  /// [`ModuleBase`], which is the whole of what the type asks.
  pub(crate) fn of(position: BytePos, base: ModuleBase) -> Self {
    debug_assert!(
      position >= base.0,
      "a position at {:?} precedes the base of the module holding it, {:?}",
      position,
      base.0
    );

    Self::clamped(position, base)
  }

  /// The arithmetic [`Self::of`] performs once its invariant holds.
  ///
  /// Separate from the assertion so that every profile can test the answer a
  /// release build gives. The assertion compiles out of release, so a debug
  /// case that called [`Self::of`] with a position below the base would panic
  /// rather than read the clamp, and the clamp is the shipped behaviour.
  ///
  /// Call [`Self::of`], not this. This one skips the invariant, and an offset
  /// taken with the invariant skipped is the "rank by earliest in the file"
  /// failure the type exists to prevent. It is crate-visible only because the
  /// suite sits in a sibling module and cannot reach a private item.
  pub(crate) fn clamped(position: BytePos, base: ModuleBase) -> Self {
    Self(position.0.saturating_sub(base.0.0))
  }

  /// How far apart two offsets are. Defined only between offsets, which is what
  /// makes a cross-`SourceMap` comparison unspellable.
  fn distance(self, other: Self) -> u32 {
    self.0.abs_diff(other.0)
  }

  /// An offset stated directly, for the cases that spell a query out rather
  /// than reading one off a call.
  ///
  /// Test-only, and deliberately so: production has a `BytePos` and a base, and
  /// the whole point of the type is that it must supply both.
  #[cfg(test)]
  pub(crate) fn at(offset: u32) -> Self {
    Self(offset)
  }
}

/// Every authored position a style namespace key could be resolved to, in the
/// parsed source of one module, collected in a single walk.
///
/// The debug path asks for the authored position of *every* namespace of every
/// `stylex.create` call in the file, and the question is always the same shape:
/// which object literal in the source spells this key. Answering it by walking
/// the module per namespace is `O(namespaces x file size)`, which is quadratic
/// in the size of a file that is one long list of styles -- and that is what a
/// `dev` build of one spends most of its time doing. One walk builds this
/// instead, and each lookup becomes a hash hit followed by a comparison of the
/// handful of candidates that actually spell the key.
///
/// Built from the memoized parsed source and discarded with it, so a candidate
/// span always belongs to the module the caller is resolving against.
#[derive(Clone, Debug)]
pub struct KeySpanIndex {
  by_key: FxHashMap<Atom, Vec<IndexedCandidate>>,
  /// Where the indexed module starts, so a candidate's position can be recorded
  /// as an offset into its own file rather than into a source map the query side
  /// does not share.
  base: ModuleBase,
}

/// One object literal that spells a given key: where the key is written, and
/// what stands beside it for disambiguation.
#[derive(Clone, Debug)]
struct IndexedCandidate {
  /// The span of the key itself -- the answer a lookup returns.
  span: Span,
  /// The keys of the namespace's own value object, empty when the value is not
  /// an object literal.
  namespace_value_keys: Vec<Atom>,
  /// Every namespace key of the containing object argument, shared between that
  /// object's candidates because they all rank against the same siblings.
  sibling_keys: Rc<Vec<Atom>>,
  /// Where the candidate's call is written, for the distance tie-break.
  candidate_offset: FileOffset,
  /// The callee the candidate's object was written as an argument to, shared
  /// between that object's candidates.
  ///
  /// This index walks *every* call with an object first argument, because
  /// narrowing it to StyleX would mean teaching it the module's import
  /// bindings. Walking them is cheap; letting them compete is not. A
  /// `useMemo({ root: ... })` beside a `stylex.create({ root: ... })` ties on
  /// both overlap fields, and a tie refuses the lookup -- sending it back to the
  /// whole-module value walk this index exists to replace.
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

  /// The authored span the `query` names, or [`DUMMY_SP`] when no candidate
  /// spells its key or when two candidates are equally good.
  ///
  /// Two namespaces in one file can spell the same key, so a match on the key
  /// alone is not an answer. Candidates are ranked by how much of the compiled
  /// call they reproduce -- see [`CandidateRank`] -- and a tie is refused rather
  /// than guessed, because a wrong `file:line` is worse than none.
  pub fn resolve(&self, query: &NamespaceKeyQuery) -> Span {
    // Interned rather than borrowed: an `Atom` key cannot be looked up by
    // `&str`, and interning one name is nothing against the walk this replaces.
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

  /// Records every namespace key of one call's object argument.
  ///
  /// The candidate a key resolves to is its *last* occurrence in the object,
  /// which is the property a runtime object literal would keep, and one
  /// candidate per (object, key) pair rather than per property -- so a key
  /// written twice in one object is not read as two objects disagreeing about
  /// where it lives.
  ///
  /// Last occurrence for the value keys too, and that is a deliberate change
  /// from the walk this replaces: that one moved its answer to the later
  /// property but kept the earlier property's value keys when the later value
  /// was not an object literal, so `{ root: { color: 'red' }, root: someVar }`
  /// ranked as though `root` still spelled `color`. The surviving property is
  /// the one the compiled call was built from, so it is the one whose value can
  /// be compared against it.
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

    // Iterated out of a hash map, and each candidate list is still in a
    // deterministic order: `in_this_object` is keyed by name, so one object
    // contributes at most one candidate per name and the shuffled order is the
    // order *different* names are appended to *different* lists in. Within one
    // name the order is the visit order, which is the source order.
    //
    // What that buys is that `resolve` may rank candidates without a tiebreak
    // on position. It does not rely on it -- a tie there sets `ambiguous` and a
    // strict improvement clears it, in any order -- so keep it that way: a
    // "first best wins" shortcut would make the answer depend on this order,
    // and this order is only stable per name, not across the map.
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

/// How well a candidate matches the namespace being placed, higher being
/// better.
///
/// The ordering is derived, so the field order *is* the precedence: how much of
/// the namespace's own value the candidate reproduces, then how many of the
/// call's other namespace keys it spells, then how close it is written to the
/// compiled call. The distance is `Reverse`d so a nearer candidate wins.
///
/// `None` sorts below `Some`, so `Reverse(None)` outranks every measured
/// distance -- the opposite of what an earlier reading of this said. It never
/// decides anything, because the `Option` comes from the *query* rather than
/// from the candidate and so is uniform across one `resolve`: a call with no
/// position of its own leaves every candidate tied here, and the two overlap
/// fields above decide alone.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct CandidateRank {
  /// Whether the candidate was written as an argument to the call being placed.
  /// First, because an object handed to some *other* function is not a worse
  /// answer than the right one -- it is not an answer at all.
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
      // Both sides are [`FileOffset`], which is the only thing that can be
      // compared here and the only thing either side can hold -- see that type
      // for what happens when a raw `BytePos` is compared instead.
      distance_from_target: Reverse(
        query
          .target_offset
          .map(|target_offset| self.candidate_offset.distance(target_offset)),
      ),
    }
  }
}

/// How many of `authored` the compiled call also spells. A key written twice is
/// counted twice, because it really is two properties.
fn overlap(authored: &[Atom], compiled: &FxHashSet<Atom>) -> usize {
  authored
    .iter()
    .filter(|name| compiled.contains(*name))
    .count()
}

/// One namespace of one compiled `stylex.create` call, described the way the
/// index ranks candidates against: the key to find, what else the call spells,
/// and where the call itself sits.
///
/// Read from the *compiled* call, which is why none of it can be taken for
/// granted: shorthand expansion has already rewritten the values, and a
/// synthesized call carries no position at all.
#[derive(Clone, Debug)]
pub struct NamespaceKeyQuery<'a> {
  pub namespace_key: &'a str,
  /// The namespace keys of the call's object argument, this one included.
  pub(crate) sibling_keys: Rc<FxHashSet<Atom>>,
  /// The keys of this namespace's own value object.
  pub namespace_value_keys: FxHashSet<Atom>,
  /// Where the call's object argument starts, for the proximity tie-break.
  pub target_offset: Option<FileOffset>,
  /// The callee of the call being placed, so an object argument to a different
  /// function cannot answer for it.
  pub(crate) callee: Option<&'a Callee>,
}

/// Everything a key-span lookup needs that belongs to the *call* rather than to
/// one of its namespaces.
///
/// Every namespace of one `stylex.create` shares its sibling keys, its proximity
/// anchor, the cache-key digest built from those, and the wrapped expression the
/// value-matching fallback needs. Building any of them per namespace makes the
/// call quadratic in its own namespace count, which is what this exists to
/// prevent.
///
/// One type rather than four parameters, and that is the load-bearing part: they
/// all have to describe the *same* call. Passed separately, a caller could hand
/// over a digest built from one call beside the keys of another, and the result
/// is a wrong span written into the cache under a key that looks right. Held
/// together, the invariant is the constructor's rather than the caller's.
pub struct CallLookup<'a> {
  call_expr: &'a CallExpr,
  /// The call's object argument, resolved once. Read by every namespace's query,
  /// which is why it is not re-walked per namespace.
  object_arg: Option<&'a ObjectLit>,
  /// The namespace keys of that object argument.
  sibling_keys: Rc<FxHashSet<Atom>>,
  /// Where the object argument starts, for the proximity tie-break.
  target_offset: Option<FileOffset>,
  /// The call's half of the span cache key.
  digest: u128,
  /// The call wrapped as an expression, built on first use.
  ///
  /// Only the value-matching fallback and a cache miss need it, and it is a deep
  /// clone of the whole call -- so a call whose namespaces all resolve through
  /// the input source map, or all hit the span cache, never pays for it.
  wrapped: OnceCell<Expr>,
}

impl<'a> CallLookup<'a> {
  /// `module_base` is where the module holding `call_expr` starts, which is what
  /// turns the call's `BytePos` into an offset the index can be compared
  /// against. The two are positioned in different source maps -- see `rank`.
  ///
  /// Optional, and absent means "no proximity signal" rather than "measured
  /// from zero" -- see [`ModuleBase`]. Without one every candidate ties on
  /// distance and the overlap fields decide.
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

  /// The call's half of a span cache key, mixed with each namespace's half.
  pub fn digest(&self) -> u128 {
    self.digest
  }

  /// The call as an expression, cloned on the first caller that needs one.
  pub fn wrapped(&self) -> &Expr {
    self
      .wrapped
      .get_or_init(|| Expr::Call(self.call_expr.clone()))
  }

  /// A lookup for one of this call's namespaces.
  pub fn query(&self, namespace_key: &'a str) -> NamespaceKeyQuery<'a> {
    NamespaceKeyQuery {
      namespace_key,
      sibling_keys: Rc::clone(&self.sibling_keys),
      // The one genuinely per-namespace signal: the keys of *this* namespace's
      // own value object.
      namespace_value_keys: self
        .object_arg
        .map(|object| namespace_value_keys(object, namespace_key))
        .unwrap_or_default(),
      target_offset: self.target_offset,
      callee: Some(&self.call_expr.callee),
    }
  }
}

/// The call's contribution to a span cache key.
///
/// Kept beside the state it hashes so the two cannot drift: a field added to
/// [`CallLookup`] that belongs in the key is added here, and nowhere else asks.
fn call_digest(
  call_expr: &CallExpr,
  object_arg: Option<&ObjectLit>,
  sibling_keys: &FxHashSet<Atom>,
) -> u128 {
  // Sorted, because a `FxHashSet`'s iteration order is not part of the identity
  // being keyed -- two calls with the same keys in a different order are the
  // same call.
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

/// The call's first argument, when it is an object literal -- the only shape
/// either side of this index reads, because that is what `stylex.create` takes.
fn first_object_arg(call_expr: &CallExpr) -> Option<&ObjectLit> {
  match call_expr.args.first().map(|arg| arg.expr.as_ref()) {
    Some(Expr::Object(object)) => Some(object),
    _ => None,
  }
}

/// Where `object` is written, or `None` when nothing wrote it: a synthesized
/// node carries `DUMMY_SP`, and byte zero would sort before every authored
/// position rather than mean "unknown".
fn object_lo(object: &ObjectLit) -> Option<BytePos> {
  (!object.span.is_dummy()).then_some(object.span.lo)
}

/// The keys of `namespace_key`'s own value object in `object`, empty when the
/// object does not spell the key or does not bind it to an object literal.
///
/// The first such property wins, unlike the index side's last -- this reads the
/// compiled call, where the namespace was built from a map and cannot repeat.
fn namespace_value_keys(object: &ObjectLit, namespace_key: &str) -> FxHashSet<Atom> {
  // `find` then match, rather than one `find_map`: the closure there answered
  // `None` both for a property that does not name the namespace and for one
  // that names it without an object value, so a *later* property of the same
  // name could still be matched -- which is not "the first such property wins".
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

/// The literal keys of `value` when it is an object literal, and none when it is
/// anything else -- a namespace bound to a reference or a call contributes no
/// keys to compare.
fn object_lit_keys(value: &Expr) -> impl Iterator<Item = Atom> + '_ {
  let object = match value {
    Expr::Object(object) => Some(object),
    _ => None,
  };

  object.into_iter().flat_map(collect_object_lit_keys)
}
