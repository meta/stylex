use super::*;
use crate::growable_stack;
use stylex_constants::constants::evaluation_errors::expression_too_deep;
use swc_core::common::EqIgnoreSpan;

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
pub(crate) trait Memoized: Sized {
  /// A remembered answer, read back as this caller's own value.
  fn from_memo(remembered: EvaluateResultValue) -> Self;

  /// This value, as the one the memo stores.
  fn to_memo(&self) -> EvaluateResultValue;

  fn can_memoize(&self) -> bool {
    true
  }
}

/// Values that can cross the AST memo boundary.
impl Memoized for EvaluateResultValue {
  fn from_memo(remembered: Self) -> Self {
    remembered
  }

  fn to_memo(&self) -> Self {
    self.clone()
  }

  fn can_memoize(&self) -> bool {
    // Fresh containers/closures must retain per-invocation identity.
    matches!(
      self,
      Self::Expr(Expr::Lit(_) | Expr::Ident(_)) | Self::Null | Self::ThemeRef(_)
    )
  }
}

/// One level of a fold, through the ceiling and through the memo.
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

/// Charge one evaluation level before a memo lookup or descent.
fn descend_one_level<T>(
  path: &Expr,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  descend: impl FnOnce(&mut EvaluationState, &mut StateManager) -> Option<T>,
) -> Option<T> {
  let ceiling = traversal_state.evaluation_ceiling();

  if traversal_state.evaluation_depth == 0 {
    traversal_state.depth_refused = false;
  }

  if traversal_state.evaluation_depth >= ceiling {
    traversal_state.depth_refused = true;

    deopt(path, state, &expression_too_deep(ceiling));

    return None;
  }

  traversal_state.evaluation_depth += 1;

  let result = growable_stack::grown_per_level(|| descend(state, traversal_state));

  traversal_state.evaluation_depth -= 1;

  result
}

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
  // Call-time parameters, overlays and externally supplied mapper bindings must run for every call.
  if !state.functions.is_static_memo_context() {
    return fold(state, traversal_state);
  }

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
  // Retained immutable initializer roots use exact owned identity.
  use stylex_state::seen_value::{MemoContext, MemoExpressionKey};
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
  let key = SeenKey {
    subtree: expression,
    context: MemoContext {
      inventory: state.static_memo_inventory(),
      characters: traversal_state.character_ceiling(),
      entries: traversal_state.entry_ceiling(),
      depth: traversal_state.evaluation_ceiling(),
      span: path.span(),
      execution_position: stylex_js::native::execution_position(),
      host_epoch: stylex_js::native::host_epoch(),
      host_revision,
      model_epoch: stylex_js::native::model_refusal_epoch(),
      rule_epoch: traversal_state.nested_rules_filed(),
      input_revision,
    },
  };
  let existing = traversal_state.seen.get(&key);

  match existing {
    Some(evaluate_value) => {
      let evaluated_value: &SeenValue = evaluate_value.borrow();
      if matches!(key.subtree, MemoExpressionKey::SubtreeHash(_))
        && !evaluated_value
          .expression
          .as_deref()
          .is_some_and(|recorded| recorded.eq_ignore_span(path))
      {
        // The hash selects a bucket, never a semantic answer.
        return fold(state, traversal_state);
      }

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

      // The refusal record below is skipped as well, and deliberately: a fold that filed a rule and then refused has to run again too, or the rule the second holder needs is never filed.
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
        // Only primitive/shareable values reach this copy; allocating AST containers and callbacks are never cloned into the module memo.
        traversal_state.seen.entry(key).or_insert_with(|| {
          Rc::new(SeenValue {
            expression: matches!(expression, MemoExpressionKey::SubtreeHash(_))
              .then(|| Rc::new(path.clone())),
            value: val.as_ref().map(T::to_memo),
            resolved: true,
          })
        });
      } else if !state.confident && traversal_state.owns_its_refusals() {
        traversal_state.keep_evaluation_functions(Rc::clone(&state.functions));

        traversal_state.seen.entry(key).or_insert_with(|| {
          Rc::new(SeenValue {
            expression: matches!(expression, MemoExpressionKey::SubtreeHash(_))
              .then(|| Rc::new(path.clone())),
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
#[path = "tests/memo_parameter_identity_tests.rs"]
mod memo_parameter_identity_tests;

#[cfg(test)]
#[path = "tests/memo_owner_tests.rs"]
mod memo_owner_tests;

#[cfg(test)]
#[path = "tests/primitive_leaf_tests.rs"]
mod primitive_leaf_tests;

#[cfg(test)]
#[path = "tests/memo_tests.rs"]
mod memo_tests;
