use super::super::*;
use swc_core::ecma::ast::{BinExpr, BinaryOp};

/// The reason recorded for an expression that lost no confidence on either side
/// and still could not be folded.
const DEFAULT_DEOPT_REASON: &str = "unknown error";

/// One of the three operators the reference implementation keeps on a
/// logical-expression node of its own.
///
/// SWC has no such node — `||`, `&&` and `??` arrive as binary expressions
/// alongside `+` and `>>` — so the binary-expression node asks for this before
/// doing anything else, and hands the three it names over here.
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub(in super::super) enum LogicalOp {
  Or,
  And,
  Nullish,
}

impl LogicalOp {
  #[inline]
  pub(in super::super) fn of(op: BinaryOp) -> Option<Self> {
    match op {
      BinaryOp::LogicalOr => Some(Self::Or),
      BinaryOp::LogicalAnd => Some(Self::And),
      BinaryOp::NullishCoalescing => Some(Self::Nullish),
      _ => None,
    }
  }
}

/// Folds `||`, `&&` and `??` over evaluated values, returning the winning
/// operand verbatim.
///
/// Each side is evaluated under a confidence of its own, so a side the fold
/// never consults cannot deopt the expression — `token ?? 'red'` folds whether
/// or not the fallback would have.
///
/// The right side is evaluated only where the guard actually asks about it,
/// which is the short-circuit the language performs and this node's whole
/// reason for existing. It is not merely a saving: evaluating an operand is
/// what queues a `*.stylex.js` theme import for tree-shake compensation, so a
/// losing `tokens.color` evaluated anyway would leave an import behind for a
/// value that never reached the stylesheet. Which operand the fold keeps is
/// unchanged by the laziness — a left side that wins wins whatever the right
/// side would have said.
pub(in super::super) fn evaluate(
  op: LogicalOp,
  bin: &BinExpr,
  state: &mut EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Option<EvaluateResultValue> {
  let left = evaluate_operand(&bin.left, state, traversal_state, fns);

  let decision = match left.confident {
    true => decide(op, left.value.as_ref()),
    // The right side is still evaluated below, because which path the deopt
    // names depends on it: an unconfident right side is the one reported, and
    // only a confident one lets the left side's own reason through.
    false => Decision::Refuse,
  };

  if matches!(decision, Decision::Left) {
    return Some(operand_value(left.value));
  }

  let right = evaluate_operand(&bin.right, state, traversal_state, fns);

  if matches!(decision, Decision::Right) && right.confident {
    return Some(operand_value(right.value));
  }

  if !left.confident {
    return deopt(&bin.left, state, left.deopt_reason());
  }

  if !right.confident {
    return deopt(&bin.right, state, right.deopt_reason());
  }

  deopt(&Expr::Bin(bin.clone()), state, DEFAULT_DEOPT_REASON)
}

/// One side of the operator, evaluated under a confidence of its own.
struct Operand {
  value: Option<EvaluateResultValue>,
  confident: bool,
  reason: Option<String>,
}

impl Operand {
  fn deopt_reason(&self) -> &str {
    self.reason.as_deref().unwrap_or(DEFAULT_DEOPT_REASON)
  }
}

/// Evaluates one side against a forked state, so that losing confidence there
/// says nothing about the expression as a whole until the fold has asked.
///
/// Only the confidence is forked. Everything the evaluation discovers about the
/// module — imports to queue, values to memoize — lands on `traversal_state`,
/// which is shared, and which is where those are deduplicated.
///
/// The reason is carried over rather than cleared, matching the state the
/// reference implementation forks. It cannot arrive stale: a reason is only ever
/// recorded together with the loss of confidence that produced it, and this node
/// is reached with confidence intact.
fn evaluate_operand(
  expr: &Expr,
  state: &EvaluationState,
  traversal_state: &mut StateManager,
  fns: &FunctionMap,
) -> Operand {
  let mut own_state = EvaluationState {
    confident: true,
    deopt_path: None,
    ..state.clone()
  };

  let value = evaluate_cached(expr, &mut own_state, traversal_state, fns);

  Operand {
    value,
    confident: own_state.confident,
    reason: own_state.deopt_reason,
  }
}

/// Whether `op` evaluates its right operand at all, given the value its left
/// operand holds.
///
/// The short circuit is a fact about the operator rather than about who is
/// asking, so the guard in front of the engine reads it from here rather than
/// keeping a table of its own that could come to disagree with [`decide`] about
/// which operand a build reaches.
pub(in super::super) fn evaluates_its_right_operand(
  op: LogicalOp,
  left: &EvaluateResultValue,
) -> bool {
  !matches!(decide(op, Some(left)), Decision::Left)
}

/// What the left side alone settles.
enum Decision {
  /// The left operand is the answer, whatever the right side holds.
  Left,
  /// The right operand is the answer if it evaluated confidently, and the
  /// caller deopts on it if it did not.
  Right,
  /// The guard declines to fold, and the caller deopts.
  Refuse,
}

/// The operand the reference implementation's guard lets through, decided from
/// the left side alone so the right one is evaluated only where it is consulted.
///
/// The guards are reproduced in the shape upstream writes them, including the
/// nullish one — `leftConfident && !!(left ?? rightConfident)` tests the left
/// side's *truthiness* where it evidently meant its nullishness, so `0 ?? 5`
/// refuses to fold even though both sides are confident. Correcting it here
/// would make this compiler fold a value the reference implementation leaves
/// alone, and a silent CSS difference between two builds of the same source is
/// worse than inheriting the restriction. Reported upstream rather than fixed
/// locally.
fn decide(op: LogicalOp, left: Option<&EvaluateResultValue>) -> Decision {
  match op {
    LogicalOp::Or => match truthiness(left) {
      Some(true) => Decision::Left,
      Some(false) => Decision::Right,
      None => Decision::Refuse,
    },
    LogicalOp::And => match truthiness(left) {
      Some(true) => Decision::Right,
      Some(false) => Decision::Left,
      None => Decision::Refuse,
    },
    LogicalOp::Nullish => {
      if is_nullish(left) {
        Decision::Right
      } else {
        match truthiness(left) {
          Some(true) => Decision::Left,
          Some(false) | None => Decision::Refuse,
        }
      }
    },
  }
}

/// `ToBoolean` over an operand, or `None` when its truthiness cannot be read at
/// compile time and the caller has to deopt.
///
/// An operand that evaluated confidently to no value is `undefined`, which is
/// falsy.
///
/// Upstream always holds a real JavaScript value here, so its `!!left` always
/// answers and it would fold where a refusal deopts. The refusal is kept on
/// purpose: it costs a declaration that falls to the runtime, where a guess
/// would put the wrong operand in the stylesheet.
fn truthiness(value: Option<&EvaluateResultValue>) -> Option<bool> {
  match value {
    Some(value) => evaluate_result_to_js_boolean(value),
    None => Some(false),
  }
}

/// Whether an operand is one of the two values `??` takes its right side for.
///
/// An operand that evaluated confidently to no value is `undefined`, which is
/// one of them.
fn is_nullish(value: Option<&EvaluateResultValue>) -> bool {
  match value {
    Some(value) => evaluate_result_is_nullish(value),
    None => true,
  }
}

/// The winning operand as a value, spelling a confidently absent one as the
/// `undefined` the language gives it.
fn operand_value(value: Option<EvaluateResultValue>) -> EvaluateResultValue {
  value.unwrap_or_else(js_undefined)
}

#[cfg(test)]
#[path = "tests/logical_expression_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/unfoldable_operand_tests.rs"]
mod unfoldable_operand_tests;
