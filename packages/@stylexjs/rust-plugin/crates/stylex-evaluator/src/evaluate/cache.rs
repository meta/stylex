use super::*;
use crate::growable_stack;
use stylex_constants::constants::evaluation_errors::expression_too_deep;

pub(crate) fn evaluate_cached(
  path: &Expr,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Option<EvaluateResultValue> {
  if !state.confident {
    return None;
  }
  let normalized = normalize_expr(path);
  if matches!(
    normalized,
    Expr::Lit(Lit::Str(_) | Lit::Num(_) | Lit::Bool(_) | Lit::Null(_))
  ) {
    // A primitive leaf consumes work, not another recursive AST level. Atom
    // cloning retains exact WTF-8, including lone UTF-16 surrogates.
    let bytes = traversal_state.input_source_file.as_ref().map_or_else(
      || path.span().hi.0.saturating_sub(path.span().lo.0) as usize,
      |source| source.src.len(),
    );
    return stylex_js::native::with_evaluation_sized(bytes, || {
      if traversal_state.evaluation_depth == 0 {
        traversal_state.depth_refused = false;
        stylex_js::native::set_result_limits(
          traversal_state.character_ceiling(),
          traversal_state.entry_ceiling(),
        );
      }
      if stylex_js::native::budget_exhausted() || stylex_js::native::charge_work(1).is_none() {
        return deopt(
          path,
          state,
          "Static evaluation exceeded its resource limit.",
        );
      }
      Some(EvaluateResultValue::Expr(normalized.clone()))
    });
  }
  folded_once(path, state, traversal_state, |state, traversal_state| {
    _evaluate(path, state, traversal_state, fns)
  })
}

/// A value one level of a fold can answer with, and what the memo keeps of it.
///
/// Everything the memo stores is an [`EvaluateResultValue`], because that is
/// what the tree can hold. A caller whose own value carries more than that says
/// here how it crosses in each direction -- and, by saying it, accepts that the
/// extra does not survive a remembered answer.
pub(crate) trait Memoized: Sized {
  /// A remembered answer, read back as this caller's own value.
  fn from_memo(remembered: EvaluateResultValue) -> Self;

  /// This value, as the one the memo stores.
  fn to_memo(&self) -> EvaluateResultValue;

  fn can_memoize(&self) -> bool {
    true
  }
}

/// The evaluator's own value, which is what the memo holds, so both directions
/// are the identity.
impl Memoized for EvaluateResultValue {
  fn from_memo(remembered: Self) -> Self {
    remembered
  }

  fn to_memo(&self) -> Self {
    self.clone()
  }

  fn can_memoize(&self) -> bool {
    // Fresh containers/closures must retain per-invocation identity. Their
    // binding values are shared by the guarded operation ledger, not this memo.
    matches!(
      self,
      Self::Expr(Expr::Lit(_) | Expr::Ident(_)) | Self::Null | Self::ThemeRef(_)
    )
  }
}

/// One level of a fold, through the ceiling and through the memo.
///
/// [`evaluate_cached`] is this with the evaluator's own dispatch as the fold.
/// The binary-expression node passes a fold of its own, because what it folds a
/// `+` to carries a measurement no expression can hold — but the level it
/// costs, the memo it reads and writes, and the refusal it records all have to
/// stay one behaviour, and that is what lives here. A second copy of it would be
/// a second answer to "how deep is this", which is the one question a memo
/// keyed by structure alone cannot be asked twice.
///
/// [`Memoized`] is how a caller's value crosses into the memo and back. One
/// carrying more than the tree can hold — a measured string — loses the extra on
/// the way through and keeps it everywhere else, which is exactly the trade: the
/// memo answers for a subtree written twice, and the measurement for a subtree
/// grown once.
pub(crate) fn folded_once<T: Memoized>(
  path: &Expr,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fold: impl FnOnce(&mut EvaluationState, &mut StateManager) -> Option<T>,
) -> Option<T> {
  let bytes = traversal_state.input_source_file.as_ref().map_or_else(
    || path.span().hi.0.saturating_sub(path.span().lo.0) as usize,
    |source| source.src.len(),
  );
  if !stylex_js::native::has_active_evaluation() {
    traversal_state.keep_evaluation_functions(state.functions.clone());
  }
  let nesting = if traversal_state.evaluation_depth == 0 {
    Some(growable_stack::nesting_of(path))
  } else {
    None
  };
  if nesting.is_some_and(|depth| !growable_stack::carriable(depth)) {
    deopt(
      path,
      state,
      "Static evaluation exceeded its resource limit.",
    );
    return None;
  }
  let operation = || {
    stylex_js::native::with_evaluation_sized(bytes, || {
      if traversal_state.evaluation_depth == 0 {
        stylex_js::native::set_result_limits(
          traversal_state.character_ceiling(),
          traversal_state.entry_ceiling(),
        );
      }
      descend_one_level(path, state, traversal_state, |state, traversal_state| {
        memoized_fold(path, state, traversal_state, fold)
      })
    })
  };
  match nesting {
    Some(depth) => growable_stack::grown_for_depth(depth, operation),
    None => operation(),
  }
}

/// The outer half of [`folded_once`]: `descend` costs a level of the evaluation
/// ceiling, or `path` is refused because there are none left.
///
/// Split from the memo so the level is charged before the structural hash runs,
/// which is where the comment below starts.
fn descend_one_level<T>(
  path: &Expr,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  descend: impl FnOnce(&mut EvaluationState, &mut StateManager) -> Option<T>,
) -> Option<T> {
  // Checked before the descent, which for the memo begins with a structural hash
  // over the same subtree -- the frame that would overflow if the budget were
  // checked after it.
  //
  // Returning here leaves *this* node out of `seen`, which is deliberate: the
  // memo is keyed by a structural hash that says nothing about the depth the
  // fold reached the node at, so recording "no" against the subtree that earned
  // it would answer for the same subtree written shallowly.
  //
  // The ancestors have to be left out for the same reason, which is what
  // `depth_refused` is for. Leaving them in was a bug: the entry a memo write
  // makes is post-order -- after `_evaluate` returns, not before it recurses --
  // so it is not the in-progress marker a cycle terminates on and nothing needs
  // it to be there. What it did instead was answer for a shallow reading of any
  // subtree the refusal happened inside, so a dynamic style deep enough to
  // refuse decided whether the *next* namespace folded, and property order
  // decided the emitted CSS.
  //
  // What the ceiling counts, given that, is the levels the fold *descends*, not
  // the levels an expression is written with: a hit answers without descending,
  // so a subtree already folded elsewhere costs one level rather than its own
  // height, and an expression can fold beside a sibling that warmed its inner
  // subtree where it would refuse alone. Charging a hit for the height it skips
  // was built and measured: the height a subtree records is the deepest the fold
  // went *anywhere* under it, which for the object of a member chain is the
  // whole object literal rather than the one member a read plucks out of it, so
  // the charge refused two of the member-chain boundaries this suite pins. Left
  // as it is because the asymmetry only ever folds *more* -- upstream has no
  // ceiling at all, so every case it decides differently is a case upstream
  // folds too, and no working input becomes a build error.
  //
  // The ceiling is read off the per-file options rather than threaded down the
  // recursion: every arm already carries the state manager, and a ceiling passed
  // through the arms instead would be a parameter each of them ignores. Where the
  // number comes from, and why the default is sized for hand-written styles
  // rather than for the deepest foldable input, is
  // `stylex_structures::evaluation_depth`.
  let ceiling = traversal_state.evaluation_ceiling();

  if traversal_state.evaluation_depth == 0 {
    // A new top-level fold. Whatever the previous one refused, its unwind is
    // over, so the frames of this one are free to record their own answers.
    traversal_state.depth_refused = false;
  }

  if traversal_state.evaluation_depth >= ceiling {
    traversal_state.depth_refused = true;

    // `deopt` answers with the evaluator's own value type, and this helper
    // answers with the caller's, so the refusal is recorded here and the `None`
    // returned separately.
    deopt(path, state, &expression_too_deep(ceiling));

    return None;
  }

  traversal_state.evaluation_depth += 1;

  let result = growable_stack::grown_per_level(|| descend(state, traversal_state));

  traversal_state.evaluation_depth -= 1;

  result
}

// Only a literal string's repeat with inert literal arguments is proven pure
// before evaluation. Alias receivers, getters and argument calls stay live.
fn inert_literal_repeat(call: &CallExpr) -> bool {
  let Callee::Expr(callee) = &call.callee else {
    return false;
  };
  let Expr::Member(member) = normalize_expr(callee) else {
    return false;
  };
  if !matches!(normalize_expr(&member.obj), Expr::Lit(Lit::Str(_))) {
    return false;
  }
  let repeat = match &member.prop {
    MemberProp::Ident(name) => name.sym == "repeat",
    MemberProp::Computed(key) => {
      matches!(normalize_expr(&key.expr), Expr::Lit(Lit::Str(name)) if name.value.as_str() == Some("repeat"))
    },
    MemberProp::PrivateName(_) => false,
  };
  repeat
    && call.args.first().is_some_and(|arg| {
      arg.spread.is_none() && matches!(normalize_expr(&arg.expr), Expr::Lit(Lit::Num(_)))
    })
    && call.args.iter().all(|arg| {
      arg.spread.is_none()
        && matches!(
          normalize_expr(&arg.expr),
          Expr::Lit(Lit::Str(_) | Lit::Num(_) | Lit::Bool(_) | Lit::Null(_))
        )
    })
}

/// The memo fold runs inside the caller's charged evaluation level.
fn memoized_fold<T: Memoized>(
  path: &Expr,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fold: impl FnOnce(&mut EvaluationState, &mut StateManager) -> Option<T>,
) -> Option<T> {
  // Calls/gets run live; identifiers reach the guarded operation binding ledger.
  // Fresh literal/local allocations cannot become one instance across frames.
  let normalized = normalize_expr(path);
  if matches!(normalized, Expr::Call(call) if !inert_literal_repeat(call))
    || matches!(
      normalized,
      Expr::Member(_)
        | Expr::Ident(_)
        | Expr::Object(_)
        | Expr::Array(_)
        | Expr::Arrow(_)
        | Expr::Fn(_)
    )
  {
    return fold(state, traversal_state);
  }
  // Retained immutable initializer roots use exact owned identity. Rebuilt
  // expressions keep the structural fallback; no transient AST address is a
  // memo key. Position, call-time context and limits remain distinct inputs.
  use std::hash::{Hash, Hasher};
  use stylex_state::seen_value::MemoExpressionKey;
  if super::member_observer::active()
    && super::member_observer::needs_events(path, stable_hash_unspanned(path))
  {
    return fold(state, traversal_state);
  }
  let input_revision = traversal_state.evaluation_input_revision();
  let host_revision = stylex_js::native::memo_host_revision();
  let expression = traversal_state.retained_initializer_site(path).map_or_else(
    || MemoExpressionKey::SubtreeHash(stable_hash_unspanned(path)),
    MemoExpressionKey::RetainedInitializer,
  );
  let mut context = rustc_hash::FxHasher::default();
  state.memo_fingerprint().hash(&mut context);
  traversal_state.character_ceiling().hash(&mut context);
  traversal_state.entry_ceiling().hash(&mut context);
  traversal_state.evaluation_ceiling().hash(&mut context);
  path.span().hash(&mut context);
  stylex_js::native::execution_position().hash(&mut context);
  stylex_js::native::host_epoch().hash(&mut context);
  host_revision.hash(&mut context);
  stylex_js::native::model_refusal_epoch().hash(&mut context);
  traversal_state.nested_rules_filed().hash(&mut context);
  input_revision.hash(&mut context);
  let key = SeenKey {
    subtree: expression,
    functions: context.finish(),
  };
  let existing = traversal_state.seen.get(&key);

  match existing {
    Some(evaluate_value) => {
      let evaluated_value: &SeenValue = evaluate_value.borrow();

      if evaluated_value.resolved {
        return evaluated_value.value.clone().map(T::from_memo);
      }

      deopt(path, state, PATH_WITHOUT_NODE);

      None
    },
    None => {
      let filings_before = traversal_state.nested_rules_filed();
      let host_before = stylex_js::native::host_epoch();
      let model_before = stylex_js::native::model_refusal_epoch();
      let val = fold(state, traversal_state);

      // A fold that filed a nested rule is not remembered. The memo answers a
      // second identical expression without folding it, so an entry here would
      // give the next `keyframes({ … })` of the module its name and none of the
      // rule behind it, and the producer holding that call would carry nothing.
      // The reference has no memo and folds every rule call, which is what this
      // keeps. Every ancestor of the call is skipped for the same reason, and
      // by the same comparison.
      //
      // The refusal record below is skipped as well, and deliberately: a fold
      // that filed a rule and then refused has to run again too, or the rule
      // the second holder needs is never filed. The cost is that such a subtree
      // is walked once more, which is what the reference pays every time.
      //
      // What this costs, measured: a module writing the same `keyframes({ … })`
      // literal inline N times now costs what N different ones cost -- 8.08 ms
      // against 11.24 ms at N = 64, both straight lines from N = 1. It cannot
      // become quadratic. The skip reaches the call and its ancestors within one
      // fold, and the depth those ancestors can reach is the evaluation ceiling,
      // 32 by default. A name bound once and read many times is untouched: the
      // binding folds once and each use reads the stored value.
      if traversal_state.evaluation_input_revision() != input_revision
        || stylex_js::native::memo_host_revision() != host_revision
        || traversal_state.nested_rules_filed() != filings_before
        || stylex_js::native::budget_exhausted()
        || stylex_js::native::host_epoch() != host_before
        || stylex_js::native::model_refusal_epoch() != model_before
      {
        return val;
      }

      if state.confident && val.as_ref().is_none_or(Memoized::can_memoize) {
        traversal_state.keep_evaluation_functions(Rc::clone(&state.functions));
        // Only primitive/shareable values reach this copy; allocating AST
        // containers and callbacks are never cloned into the module memo.
        traversal_state.seen.entry(key).or_insert_with(|| {
          Rc::new(SeenValue {
            value: val.as_ref().map(T::to_memo),
            resolved: true,
          })
        });
      } else if !state.confident && traversal_state.owns_its_refusals() {
        traversal_state.keep_evaluation_functions(Rc::clone(&state.functions));
        // Recorded so the same refusal is not re-walked, but only where the
        // refusal was about the subtree itself -- which is the one question
        // `owns_its_refusals` answers, and where the reasons it can answer `false`
        // are written down.
        traversal_state.seen.entry(key).or_insert_with(|| {
          Rc::new(SeenValue {
            value: None,
            resolved: false,
          })
        });
      }

      val
    },
  }
}

#[cfg(test)]
#[path = "tests/memo_owner_tests.rs"]
mod memo_owner_tests;

#[cfg(test)]
#[path = "tests/primitive_leaf_tests.rs"]
mod primitive_leaf_tests;

#[cfg(test)]
#[path = "tests/memo_tests.rs"]
mod memo_tests;
